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

use db::migration::Migrator;

use error::Result;

pub fn migrate(migrator: &mut Migrator) -> Result<()> {
    migrator
        .migrate("originsrv",
                 r#"CREATE SEQUENCE IF NOT EXISTS origin_package_id_seq;"#)?;
    migrator
        .migrate("originsrv",
                 r#"CREATE TABLE origin_packages (
                    id bigint PRIMARY KEY DEFAULT next_id_v1('origin_package_id_seq'),
                    origin_id bigint REFERENCES origins(id),
                    owner_id bigint,
                    name text,
                    ident text UNIQUE,
                    checksum text,
                    manifest text,
                    config text,
                    target text,
                    deps text,
                    tdeps text,
                    exposes text,
                    scheduler_sync bool DEFAULT false,
                    created_at timestamptz DEFAULT now(),
                    updated_at timestamptz
             )"#)?;
    migrator.migrate("originsrv",
                 r#"CREATE OR REPLACE FUNCTION insert_origin_package_v1 (
                    op_origin_id bigint,
                    op_owner_id bigint,
                    op_name text,
                    op_ident text,
                    op_checksum text,
                    op_manifest text,
                    op_config text,
                    op_target text,
                    op_deps text,
                    op_tdeps text,
                    op_exposes text
                 ) RETURNS SETOF origin_packages AS $$
                     BEGIN
                         RETURN QUERY INSERT INTO origin_packages (origin_id, owner_id, name, ident, checksum, manifest, config, target, deps, tdeps, exposes)
                                VALUES (op_origin_id, op_owner_id, op_name, op_ident, op_checksum, op_manifest, op_config, op_target, op_deps, op_tdeps, op_exposes)
                                RETURNING *;
                         RETURN;
                     END
                 $$ LANGUAGE plpgsql VOLATILE"#)?;
    migrator
        .migrate("originsrv",
                 r#"CREATE OR REPLACE FUNCTION get_origin_package_v1 (
                    op_ident text
                 ) RETURNS SETOF origin_packages AS $$
                    BEGIN
                        RETURN QUERY SELECT * FROM origin_packages WHERE ident = op_ident;
                        RETURN;
                    END
                    $$ LANGUAGE plpgsql STABLE"#)?;
    migrator.migrate("originsrv",
                 r#"CREATE OR REPLACE FUNCTION get_origin_package_latest_v1 (
                    op_ident text,
                    op_target text
                 ) RETURNS SETOF origin_packages AS $$
                    BEGIN
                        RETURN QUERY SELECT * FROM origin_packages WHERE ident LIKE (op_ident  || '%') AND target = op_target
                          ORDER BY ident DESC
                          LIMIT 1;
                        RETURN;
                    END
                    $$ LANGUAGE plpgsql STABLE"#)?;
    migrator.migrate("originsrv",
                     r#"CREATE OR REPLACE FUNCTION get_origin_packages_for_origin_v1 (
                    op_ident text,
                    op_limit bigint,
                    op_offset bigint
                 ) RETURNS TABLE(total_count bigint, ident text) AS $$
                    BEGIN
                        RETURN QUERY SELECT COUNT(*) OVER () AS total_count, origin_packages.ident FROM origin_packages WHERE origin_packages.ident LIKE (op_ident  || '%')
                          ORDER BY ident ASC
                          LIMIT op_limit OFFSET op_offset;
                        RETURN;
                    END
                    $$ LANGUAGE plpgsql STABLE"#)?;
    migrator.migrate("originsrv",
                     r#"CREATE OR REPLACE FUNCTION get_origin_packages_unique_for_origin_v1 (
                   op_origin text,
                   op_limit bigint,
                   op_offset bigint
                 ) RETURNS TABLE(total_count bigint, name text) AS $$
                    BEGIN
                        RETURN QUERY SELECT COUNT(*) OVER () AS total_count, origin_packages.name FROM origins INNER JOIN origin_packages ON origins.id = origin_packages.origin_id WHERE origins.name = op_origin
                          GROUP BY origin_packages.name
                          ORDER BY origin_packages.name ASC
                          LIMIT op_limit OFFSET op_offset;
                        RETURN;
                    END
                    $$ LANGUAGE plpgsql STABLE"#)?;
    migrator.migrate("originsrv",
                     r#"CREATE OR REPLACE FUNCTION search_origin_packages_for_origin_v1 (
                   op_origin text,
                   op_query text,
                   op_limit bigint,
                   op_offset bigint
                 ) RETURNS TABLE(total_count bigint, ident text) AS $$
                    BEGIN
                        RETURN QUERY SELECT COUNT(*) OVER () AS total_count, origin_packages.ident FROM origins INNER JOIN origin_packages ON origins.id = origin_packages.origin_id WHERE origins.name = op_origin and origin_packages.name LIKE ('%' || op_query || '%')
                          ORDER BY ident ASC
                          LIMIT op_limit OFFSET op_offset;
                        RETURN;
                    END
                    $$ LANGUAGE plpgsql STABLE"#)?;
    migrator.migrate("originsrv",
                     r#"CREATE OR REPLACE FUNCTION sync_packages_v1() RETURNS TABLE(account_id bigint, package_id bigint, package_ident text, package_deps text) AS $$
                    BEGIN
                        RETURN QUERY SELECT origin_packages.owner_id, origin_packages.id, origin_packages.ident, origin_packages.deps FROM origin_packages WHERE origin_packages.scheduler_sync = false;
                        RETURN;
                    END
                    $$ LANGUAGE plpgsql STABLE"#)?;
    migrator.migrate("originsrv",
                     r#"CREATE OR REPLACE FUNCTION set_packages_sync_v1(in_package_id bigint) RETURNS VOID AS $$
                    BEGIN
                        UPDATE origin_packages SET scheduler_sync = true WHERE id = in_package_id;
                    END
                    $$ LANGUAGE plpgsql VOLATILE"#)?;
    Ok(())
}
