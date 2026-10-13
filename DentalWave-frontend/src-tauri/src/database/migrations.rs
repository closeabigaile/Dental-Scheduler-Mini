pub const SUPPORTED_SCHEMA_VERSION: i64 = 3;

pub const VERSION_1: &str = r#"
CREATE TABLE schema_metadata (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  schema_version INTEGER NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE offices (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL COLLATE NOCASE UNIQUE,
  address TEXT NOT NULL DEFAULT '',
  phone_number TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE doctors (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  first_name TEXT NOT NULL DEFAULT '',
  last_name TEXT NOT NULL DEFAULT '',
  display_name TEXT NOT NULL,
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
  default_office_id INTEGER REFERENCES offices(id) ON DELETE RESTRICT,
  normal_workdays TEXT,
  color TEXT NOT NULL DEFAULT '#65a9b8',
  notes TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE assistants (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  first_name TEXT NOT NULL DEFAULT '',
  last_name TEXT NOT NULL DEFAULT '',
  display_name TEXT NOT NULL,
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
  default_office_id INTEGER REFERENCES offices(id) ON DELETE RESTRICT,
  normal_workdays TEXT,
  color TEXT NOT NULL DEFAULT '#65a9b8',
  notes TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE doctor_offices (
  doctor_id INTEGER NOT NULL REFERENCES doctors(id) ON DELETE CASCADE,
  office_id INTEGER NOT NULL REFERENCES offices(id) ON DELETE RESTRICT,
  PRIMARY KEY (doctor_id, office_id)
);

CREATE TABLE assistant_offices (
  assistant_id INTEGER NOT NULL REFERENCES assistants(id) ON DELETE CASCADE,
  office_id INTEGER NOT NULL REFERENCES offices(id) ON DELETE RESTRICT,
  PRIMARY KEY (assistant_id, office_id)
);

CREATE TABLE doctor_rotation_groups (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL COLLATE NOCASE UNIQUE,
  number_of_weeks INTEGER NOT NULL CHECK (number_of_weeks > 0),
  anchor_date TEXT NOT NULL,
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE doctor_work_rules (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  doctor_id INTEGER NOT NULL REFERENCES doctors(id) ON DELETE CASCADE,
  day_of_week TEXT NOT NULL CHECK (
    day_of_week IN ('MONDAY', 'TUESDAY', 'WEDNESDAY', 'THURSDAY', 'FRIDAY', 'SATURDAY', 'SUNDAY')
  ),
  office_id INTEGER REFERENCES offices(id) ON DELETE RESTRICT,
  work_status TEXT NOT NULL CHECK (work_status IN ('WORKING', 'NOT_WORKING')),
  recurrence_type TEXT NOT NULL CHECK (recurrence_type IN ('EVERY_WEEK', 'ROTATING')),
  rotation_group_id INTEGER REFERENCES doctor_rotation_groups(id) ON DELETE RESTRICT,
  rotation_position INTEGER,
  effective_start_date TEXT,
  effective_end_date TEXT,
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CHECK (
    (recurrence_type = 'EVERY_WEEK' AND rotation_group_id IS NULL AND rotation_position IS NULL)
    OR
    (recurrence_type = 'ROTATING' AND rotation_group_id IS NOT NULL AND rotation_position IS NOT NULL AND rotation_position > 0)
  )
);

CREATE INDEX idx_doctors_active ON doctors(active);
CREATE INDEX idx_assistants_active ON assistants(active);
CREATE INDEX idx_doctor_rules_doctor ON doctor_work_rules(doctor_id);

INSERT INTO schema_metadata (id, schema_version) VALUES (1, 1);
"#;

// Additive upgrade: existing resources and patterns retain their IDs and data.
pub const VERSION_2: &str = r#"
CREATE TABLE teams (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 name TEXT NOT NULL COLLATE NOCASE UNIQUE CHECK (length(trim(name)) > 0),
 doctor_id INTEGER NOT NULL REFERENCES doctors(id) ON DELETE RESTRICT,
 default_office_id INTEGER REFERENCES offices(id) ON DELETE RESTRICT,
 active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
 color TEXT NOT NULL DEFAULT '#65a9b8',
 notes TEXT NOT NULL DEFAULT '',
 created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE team_assistants (
 team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
 assistant_id INTEGER NOT NULL REFERENCES assistants(id) ON DELETE RESTRICT,
 PRIMARY KEY (team_id, assistant_id)
);
CREATE INDEX idx_teams_doctor ON teams(doctor_id);
CREATE INDEX idx_team_assistants_resource ON team_assistants(assistant_id);
UPDATE schema_metadata SET schema_version = 2, updated_at = CURRENT_TIMESTAMP WHERE id = 1;
"#;

pub const VERSION_3: &str = r#"
CREATE TABLE calendars (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 year INTEGER NOT NULL CHECK(year BETWEEN 1900 AND 9999),
 month INTEGER NOT NULL CHECK(month BETWEEN 1 AND 12),
 office_id INTEGER NOT NULL REFERENCES offices(id) ON DELETE RESTRICT,
 published INTEGER NOT NULL DEFAULT 0 CHECK(published IN (0,1)),
 revision INTEGER NOT NULL DEFAULT 0,
 created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 UNIQUE(year, month, office_id)
);
CREATE TABLE calendar_days (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 calendar_id INTEGER NOT NULL REFERENCES calendars(id) ON DELETE CASCADE,
 date TEXT NOT NULL,
 notes TEXT NOT NULL DEFAULT '',
 UNIQUE(calendar_id, date)
);
CREATE TABLE daily_assignments (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 day_id INTEGER NOT NULL REFERENCES calendar_days(id) ON DELETE CASCADE,
 doctor_id INTEGER REFERENCES doctors(id) ON DELETE RESTRICT,
 team_id INTEGER REFERENCES teams(id) ON DELETE RESTRICT,
 label TEXT NOT NULL DEFAULT '',
 CHECK(doctor_id IS NOT NULL OR (team_id IS NULL AND length(trim(label)) > 0))
);
CREATE TABLE assignment_assistants (
 assignment_id INTEGER NOT NULL REFERENCES daily_assignments(id) ON DELETE CASCADE,
 assistant_id INTEGER NOT NULL REFERENCES assistants(id) ON DELETE RESTRICT,
 partial_day_note TEXT NOT NULL DEFAULT '' CHECK(length(partial_day_note) <= 40),
 PRIMARY KEY(assignment_id, assistant_id)
);
CREATE TABLE calendar_generation_issues (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 calendar_id INTEGER NOT NULL REFERENCES calendars(id) ON DELETE CASCADE,
 date TEXT NOT NULL,
 code TEXT NOT NULL,
 doctor_id INTEGER,
 message TEXT NOT NULL
);
CREATE INDEX idx_days_date ON calendar_days(date);
CREATE INDEX idx_assignments_day ON daily_assignments(day_id);
CREATE INDEX idx_assignment_doctor ON daily_assignments(doctor_id);
CREATE INDEX idx_assignment_assistant ON assignment_assistants(assistant_id);
UPDATE schema_metadata SET schema_version = 3, updated_at = CURRENT_TIMESTAMP WHERE id = 1;
"#;
