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

use std::fs::File;
use std::io::Write;
use chrono::prelude::*;

pub struct Logger {
    file: File,
}

impl Logger {
    pub fn init() -> Self {
        let filepath = "/tmp/builder-scheduler.log";
        Logger { file: File::create(filepath).expect("Failed to initialize stdout log file") }
    }

    pub fn log(&mut self, msg: &str) {
        let dt: DateTime<UTC> = UTC::now();
        let fmt_msg = format!("{}, {}\n", dt.format("%Y-%m-%d %H:%M:%S"), msg);

        self.file
            .write_all(fmt_msg.as_bytes())
            .expect(&format!("Logger unable to write to {:?}", self.file));
    }
}

impl Drop for Logger {
    fn drop(&mut self) {
        self.file.sync_all().expect("Unable to sync log file");
    }
}
