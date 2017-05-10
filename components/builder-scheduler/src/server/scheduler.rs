// Copyright (c) 2016-2017 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::sync::mpsc;
use std::thread::{self, JoinHandle};

use chrono::prelude::*;
use protobuf::{parse_from_bytes, Message};
use hab_net::server::ZMQ_CONTEXT;
use hab_net::routing::Broker;
use zmq;

use protocol::jobsrv::{self, Job, JobSpec};
use protocol::originsrv::*;
use protocol::scheduler as proto;
use data_store::DataStore;
use error::{Result, Error};

use server::logger::Logger;

const SCHEDULER_ADDR: &'static str = "inproc://scheduler";
const STATUS_ADDR: &'static str = "inproc://scheduler-status";

pub struct ScheduleClient {
    socket: zmq::Socket,
    status_sock: zmq::Socket,
}

impl ScheduleClient {
    pub fn connect(&mut self) -> Result<()> {
        try!(self.socket.connect(SCHEDULER_ADDR));
        try!(self.status_sock.connect(STATUS_ADDR));
        Ok(())
    }

    pub fn notify_work(&mut self) -> Result<()> {
        try!(self.socket.send(&[1], 0));
        Ok(())
    }

    pub fn notify_status(&mut self, job: &Job) -> Result<()> {
        try!(self.status_sock.send(&job.write_to_bytes().unwrap(), 0));
        Ok(())
    }
}

impl Default for ScheduleClient {
    fn default() -> ScheduleClient {
        let socket = (**ZMQ_CONTEXT).as_mut().socket(zmq::DEALER).unwrap();
        socket.set_sndhwm(1).unwrap();
        socket.set_linger(0).unwrap();
        socket.set_immediate(true).unwrap();

        let status_sock = (**ZMQ_CONTEXT).as_mut().socket(zmq::DEALER).unwrap();
        status_sock.set_sndhwm(1).unwrap();
        status_sock.set_linger(0).unwrap();
        status_sock.set_immediate(true).unwrap();

        ScheduleClient {
            socket: socket,
            status_sock: status_sock,
        }
    }
}

pub struct ScheduleMgr {
    datastore: DataStore,
    work_sock: zmq::Socket,
    status_sock: zmq::Socket,
    schedule_cli: ScheduleClient,
    msg: zmq::Message,
    logger: Logger,
}

impl ScheduleMgr {
    pub fn new(datastore: DataStore) -> Result<Self> {
        let work_sock = try!((**ZMQ_CONTEXT).as_mut().socket(zmq::DEALER));
        try!(work_sock.set_rcvhwm(1));
        try!(work_sock.set_linger(0));
        try!(work_sock.set_immediate(true));

        let status_sock = try!((**ZMQ_CONTEXT).as_mut().socket(zmq::DEALER));
        try!(status_sock.set_rcvhwm(1));
        try!(status_sock.set_linger(0));
        try!(status_sock.set_immediate(true));

        let msg = try!(zmq::Message::new());
        let mut schedule_cli = ScheduleClient::default();
        try!(schedule_cli.connect());

        let logger = Logger::init();

        Ok(ScheduleMgr {
               datastore: datastore,
               work_sock: work_sock,
               status_sock: status_sock,
               schedule_cli: schedule_cli,
               msg: msg,
               logger: logger,
           })
    }

    pub fn start(ds: DataStore) -> Result<JoinHandle<()>> {
        let (tx, rx) = mpsc::sync_channel(1);
        let handle = thread::Builder::new()
            .name("scheduler".to_string())
            .spawn(move || {
                       let mut schedule_mgr = Self::new(ds).unwrap();
                       schedule_mgr.run(tx).unwrap();
                   })
            .unwrap();
        match rx.recv() {
            Ok(()) => Ok(handle),
            Err(e) => panic!("scheduler thread startup error, err={}", e),
        }
    }

    fn run(&mut self, rz: mpsc::SyncSender<()>) -> Result<()> {
        try!(self.work_sock.bind(SCHEDULER_ADDR));
        try!(self.status_sock.bind(STATUS_ADDR));

        let mut status_sock = false;
        let mut work_sock = false;
        rz.send(()).unwrap();
        loop {
            {
                let mut items = [self.work_sock.as_poll_item(1),
                                 self.status_sock.as_poll_item(1)];
                try!(zmq::poll(&mut items, -1));

                if (items[0].get_revents() & zmq::POLLIN) > 0 {
                    work_sock = true;
                }
                if (items[1].get_revents() & zmq::POLLIN) > 0 {
                    status_sock = true;
                }
            }

            if work_sock {
                if let Err(err) = self.process_work() {
                    warn!("Unable to process work: err {:?}", err);
                }
                work_sock = false;
            }

            if status_sock {
                if let Err(err) = self.process_status() {
                    warn!("Unable to process status: err {:?}", err);
                }
                status_sock = false;
            }
        }
    }

    fn process_work(&mut self) -> Result<()> {
        loop {
            // Take one group from the pending list
            let mut groups = self.datastore.pending_groups(1)?;

            // 0 means there are no pending groups, so we should consume our notice that we have work
            if groups.len() == 0 {
                info!("No more pending groups - exiting process_work");
                try!(self.work_sock.recv(&mut self.msg, 0));
                break;
            }

            // This unwrap is fine, because we just checked our length
            let group = groups.pop().unwrap();
            info!("Got pending group {}", group.get_id());
            assert!(group.get_state() == proto::GroupState::Dispatching);

            self.dispatch_group(group)?;
        }
        Ok(())
    }

    fn dispatch_group(&mut self, group: proto::Group) -> Result<()> {
        let dispatchable = self.dispatchable_projects(&group)?;
        for project in dispatchable {
            debug!("Dispatching project: {:?}", project.get_name());
            self.logger
                .log(&format!("D, {}, {},", group.get_id(), project.get_name()));

            assert!(project.get_state() == proto::ProjectState::NotStarted);

            match self.schedule_job(group.get_id(), project.get_name()) {
                Ok(job) => {
                    self.datastore.set_group_job_state(&job).unwrap();
                }
                Err(err) => {
                    warn!("Failed to schedule job for {}, err: {:?}",
                          project.get_name(),
                          err);

                    self.datastore
                        .set_group_state(group.get_id(), proto::GroupState::Failed)?;
                    // TBD? set_project_state(group.get_id(), project.get_name(), proto::ProjectState::Failure)?;

                    self.logger
                        .log(&format!("S, {},, {:?}", group.get_id(), proto::GroupState::Failed));
                    break;
                }
            }
        }
        Ok(())
    }

    fn dispatchable_projects(&mut self, group: &proto::Group) -> Result<Vec<proto::Project>> {
        let mut projects = Vec::new();
        for project in group
                .get_projects()
                .into_iter()
                .filter(|x| x.get_state() == proto::ProjectState::NotStarted) {
            // Check the deps for the project. If we don't find any dep that
            // is in our project list and needs to be built, we can dispatch the project.
            let package = self.datastore.get_package(&project.get_ident())?;
            let deps = package.get_deps();

            let mut dispatchable = true;
            for dep in deps {
                let parts: Vec<&str> = dep.split("/").collect();
                assert!(parts.len() >= 2);
                let name = format!("{}/{}", parts[0], parts[1]);

                if !self.check_dispatchable(group, &name) {
                    dispatchable = false;
                    break;
                };
            }

            if dispatchable {
                projects.push(project.clone());
            }
        }
        Ok(projects)
    }

    fn check_dispatchable(&mut self, group: &proto::Group, name: &str) -> bool {
        for project in group.get_projects() {
            if (project.get_name() == name) &&
               (project.get_state() != proto::ProjectState::Success) {
                return false;
            }
        }
        true
    }

    fn schedule_job(&mut self, group_id: u64, project_name: &str) -> Result<Job> {
        let mut project_get = OriginProjectGet::new();

        project_get.set_name(String::from(project_name));

        let mut conn = Broker::connect().unwrap();
        let project = match conn.route::<OriginProjectGet, OriginProject>(&project_get) {
            Ok(project) => project,
            Err(err) => {
                warn!("Unable to retrieve project: {:?}, error: {:?}",
                      project_name,
                      err);

                return Err(Error::ProtoNetError(err));
            }
        };

        let mut job_spec: JobSpec = JobSpec::new();
        job_spec.set_owner_id(group_id);
        job_spec.set_project(project);

        match conn.route::<JobSpec, Job>(&job_spec) {
            Ok(job) => {
                debug!("Job created: {:?}", job);
                Ok(job)
            }
            Err(err) => {
                warn!("Job creation error: {:?}", err);
                Err(Error::ProtoNetError(err))
            }
        }
    }

    fn process_status(&mut self) -> Result<()> {
        try!(self.status_sock.recv(&mut self.msg, 0));
        let job: Job = try!(parse_from_bytes(&self.msg));

        let build_start = match DateTime::parse_from_rfc3339(job.get_build_started_at()) {
            Ok(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
            Err(_) => String::from(""),
        };

        let build_stop = match DateTime::parse_from_rfc3339(job.get_build_finished_at()) {
            Ok(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
            Err(_) => String::from(""),
        };

        self.logger
            .log(&format!("J, {}, {}, {:?}, {}, {}",
                          job.get_owner_id(),
                          job.get_project().get_name(),
                          job.get_state(),
                          build_start,
                          build_stop));

        match self.datastore.set_group_job_state(&job) {
            Ok(_) => {
                match job.get_state() {
                    jobsrv::JobState::Complete |
                    jobsrv::JobState::Rejected |
                    jobsrv::JobState::Failed => self.update_group_state(job.get_owner_id())?,
                    _ => (),
                }
            }
            Err(err) => debug!("Did not set job state: {:?}", err),
        }

        Ok(())
    }

    fn update_group_state(&mut self, group_id: u64) -> Result<()> {
        let mut msg: proto::GroupGet = proto::GroupGet::new();
        msg.set_group_id(group_id);

        let group = self.datastore.get_group(&msg).unwrap().unwrap(); // we know the group exists

        // Group state transition rules:
        // |   Start Group State     |  Projects State  |   New Group State   |
        // |-------------------------|------------------|---------------------|
        // |     Pending             |     N/A          |        N/A          |
        // |     Dispatching         |   any Failure    |      Failed         |
        // |     Dispatching         |   all Success    |      Complete       |
        // |     Dispatching         |   otherwise      |      Pending        |
        // |     Complete            |     N/A          |        N/A          |
        // |     Failed              |     N/A          |        N/A          |

        if group.get_state() == proto::GroupState::Dispatching {
            let mut failed = 0;
            let mut succeeded = 0;
            for project in group.get_projects() {
                match project.get_state() {
                    proto::ProjectState::Failure => failed = failed + 1,
                    proto::ProjectState::Success => succeeded = succeeded + 1,
                    _ => (),
                }
            }

            let new_state = if failed > 0 {
                proto::GroupState::Failed
            } else if succeeded == group.get_projects().len() {
                proto::GroupState::Complete
            } else {
                proto::GroupState::Pending
            };

            self.datastore.set_group_state(group_id, new_state)?;
            self.logger
                .log(&format!("S, {},, {:?}", group_id, new_state));

            if new_state == proto::GroupState::Pending {
                try!(self.schedule_cli.notify_work());
            }
        } else {
            error!("Unexpected group state {:?} for group id: {}",
                   group.get_state(),
                   group_id);
        }

        Ok(())
    }
}
