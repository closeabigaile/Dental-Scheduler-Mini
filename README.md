# Dental Scheduler Mini

Dental Scheduler Mini is a focused, manager-operated scheduling application for an orthodontic office. It was derived from the larger DentalWave project and reduced to the workflows needed to prepare, review, publish, and print monthly doctor/assistant schedules.

The primary distribution is a self-contained Windows application that opens in the user's normal web browser. It runs only on the local computer and stores its data in an embedded H2 database, avoiding a hosted database subscription.

## Purpose and audience

The application is designed for the office manager or administrator responsible for coordinating assistants across doctors and office locations. It provides a repeatable starting point for each month while keeping the generated schedule editable when real-world exceptions occur.

Dental Scheduler Mini is intended to:

- keep doctor work patterns separate from individual monthly schedules;
- distribute active, office-eligible assistants across doctor teams;
- prevent obvious same-day cross-office conflicts;
- let a manager adjust individual dates without rewriting recurring rules; and
- produce a compact Monday-through-Thursday schedule for printing.

The active web interface accepts manager and administrator accounts. Doctors and assistants are scheduling resources and do not need their own login accounts in the portable edition.

## Implemented features

### Doctors and assistants

- Add, edit, activate/deactivate, and remove scheduling resources.
- Assign a default office and one or more eligible offices.
- Record normal workdays, colors, and notes.
- Keep doctors and assistants as local scheduling records rather than application users.

### Doctor work patterns

- Configure working and non-working rules by weekday.
- Assign a doctor to an office for a recurring weekday.
- Use an `EVERY_WEEK` rule or an anchored 2- to 12-week rotation.
- Limit rules with optional effective start and end dates.
- Preview resolved doctor assignments for the next four weeks.
- Reject overlapping active rules for the same doctor and pattern position.

### Reusable teams

- Create named doctor/assistant team templates.
- Assign a default office, color, notes, and active status.
- Edit, duplicate, and delete templates.

Reusable teams are currently managed independently from monthly generation; generating a month does not automatically apply a saved team template.

### Monthly schedules

- Generate a draft calendar for each configured office.
- Create daily doctor teams from the active doctor rules.
- Randomly distribute active assistants who are eligible for the office.
- Avoid assigning the same local assistant to more than one office on the same date during generation.
- Regenerate and redistribute an unpublished monthly draft.
- Add or remove an office location for a single date.
- Create, rename, and remove daily teams.
- Add and remove assistant assignments.
- Add day notes and assignment-specific partial-day notes such as `out@2`.
- Save a draft, publish a completed month, or return a published month to draft status.

Before publication, the backend checks for empty non-placeholder teams, duplicate assignments, inactive or invalid resources, office mismatches, and same-day double-booking. Retained account-backed employee assignments are also checked for approved time-off conflicts.

### Calendar and printing

- Review schedules in a month view or a published universal calendar.
- Filter published schedules by location.
- Open a dedicated print preview.
- Adjust assistant-name sizing.
- Print a Monday-through-Thursday calendar on US Letter landscape paper.

## Typical workflow

1. Start Dental Scheduler Mini and sign in as the shared scheduling user.
2. Add the office's doctors and assistants and configure their eligible locations.
3. Define each doctor's weekly or rotating office pattern.
4. Optionally maintain reusable team templates for reference and repeated team setup.
5. Choose a month and generate the office calendars.
6. Review the generated doctor teams and assistant distribution.
7. Make date-specific changes, assignments, and notes.
8. Resolve any validation issues and publish the month.
9. Preview and print the final schedule.
10. Close the portable application with the supplied launcher so it shuts down cleanly and creates a backup.

## Screenshots

### Login

![DentalWave portable edition login](docs/Loginpage.png)

### Manager dashboard

![DentalWave portable manager dashboard](docs/portableMenupage.png)

## Architecture

The portable application packages the React frontend inside the Spring Boot JAR. The browser, API, and database all run locally:

```text
Manager's browser
       |
       v
Spring Boot application on 127.0.0.1:8080
  |-- packaged React manager interface
  |-- JWT authentication and scheduling API
  `-- Spring Data JPA / Hibernate
                 |
                 v
       local file-backed H2 database
```

The portable profile binds the server to the loopback interface, so it is available only on the computer running it. It does not expose the scheduler to other computers on the office network.

The backend still contains account-backed employee, time-off, and notification modules inherited from the larger DentalWave application. Current schedule generation and publication validation share parts of that model, although those workflows are not exposed as active Mini pages.

## Technology

- **Frontend:** React 19, React Router, Axios, Vite
- **Backend:** Java 21, Spring Boot 3.4, Spring Web, Spring Security, Spring Data JPA
- **Portable storage:** file-backed H2
- **Authentication:** JWT and BCrypt password hashing
- **Packaging:** Maven Wrapper, npm, Bash, Windows Batch, and PowerShell
- **Desktop reconstruction:** Tauri 2 shell with the existing React frontend
- **Testing:** JUnit 5, Spring Boot Test, Mockito, Node's test runner, and Python `unittest`

## Desktop reconstruction (Phase 1)

A Tauri desktop shell is being developed alongside the existing Spring Boot/H2 application. Phase 1 packages the current React interface into a native desktop window, but it does not replace the REST backend, authentication, scheduling logic, or H2 storage. Tauri mode deliberately rejects REST requests, so the desktop shell cannot open or modify office data during this phase.

Tauri development requires Node.js/npm, Rust, and the platform prerequisites listed in the [official Tauri prerequisites](https://v2.tauri.app/start/prerequisites/). From `DentalWave-frontend`:

```bash
npm ci
npm run tauri:dev
```

Other useful frontend commands are:

```bash
npm run dev          # React/Vite in a browser
npm run build        # React production assets
npm run tauri:build  # Native desktop build for the current operating system
```

The existing Spring Boot/H2 application remains available through the source and Windows portable workflows documented below. Authenticated scheduler screens remain unavailable in the Phase 1 Tauri build because their REST services have not yet been replaced; run the existing web application separately when reference behavior is needed.

## Windows portable version

The portable package is designed for Windows x64 and includes:

- the packaged application JAR;
- a bundled Eclipse Temurin Java 21 runtime;
- `OPEN DENTALWAVE.bat` and `CLOSE DENTALWAVE.bat` launchers;
- backup and diagnostic tools;
- local data, log, and backup folders; and
- first-run setup for the shared `user` account.

On first launch, the manager creates a password of at least 12 characters. The launcher generates the JWT and shutdown-control secrets locally, starts the application, verifies the API and packaged browser assets, and opens `http://127.0.0.1:8080`.

Always close the application with `CLOSE DENTALWAVE.bat` before ejecting or moving the portable folder.

### Local data and backups

Portable data is stored inside the distribution:

```text
DentalWave-Portable/
`-- OtherInfo/
    |-- data/dentalwave.mv.db
    |-- backups/
    `-- logs/
```

A clean close copies the H2 database into the backup folder and retains the seven newest dated backups. The backup tool refuses to copy the database while a verified DentalWave instance is running.

Passwords are stored as BCrypt hashes. JWT and control secrets are generated into local files rather than committed to the repository.

## Build the Windows portable package

Build prerequisites:

- Java 21
- Node.js `^20.19.0` or `>=22.12.0`
- npm
- Python 3
- Bash, `curl`, and `unzip`

From the repository root:

```bash
cd DentalWave-frontend
npm ci
cd ..
./build-portable.sh
```

The build script:

1. runs the Windows launcher path tests;
2. builds the React frontend;
3. packages the frontend into the Spring Boot JAR;
4. downloads and caches a Windows x64 Java 21 runtime; and
5. creates `dist/DentalWave-Portable/`.

The Maven packaging step skips backend tests, so run the verification commands separately before distributing a build.

## Run locally with portable storage

The same portable profile can be run from source for development. The following Bash example creates a repository-local data folder; keep that folder out of commits:

```bash
cd DentalWave-frontend
npm ci
npm run build

cd ../DentalWave
mkdir -p .local-data logs
export JWT_SECRET="$(openssl rand -base64 48)"
export PORTABLE_CONTROL_TOKEN="$(openssl rand -base64 32)"
export PORTABLE_MANAGER_PASSWORD="choose-a-password-with-12-or-more-characters"
export DENTALWAVE_DATA_PATH="$PWD/.local-data/dentalwave"
export DENTALWAVE_PORTABLE_DATA_PATH="$PWD/.local-data"
./mvnw spring-boot:run -Dspring-boot.run.profiles=portable
```

Open `http://127.0.0.1:8080` and sign in with username `user` and the first-run password. Do not commit local passwords, secrets, or database files.

The default Spring profile still supports PostgreSQL-backed development through an ignored `DentalWave/src/main/resources/application-local.properties`, but PostgreSQL is not required by the portable H2 distribution.

## Verification

Build the frontend before running backend integration tests because the Spring application expects the packaged `static/index.html` resource.

```bash
cd DentalWave-frontend
npm ci
npm test
npm run lint
npm run build

cd ../DentalWave
./mvnw test

cd ..
python3 -m unittest discover -s portable/tests -p 'test_*.py'
```

The repository contains backend controller, service, repository, security, scheduling-rule, and portable-workflow tests. Frontend automation currently covers date and print utilities rather than full browser workflows.

## Project structure

```text
Dental-Scheduler-Mini/
|-- DentalWave/             Spring Boot API, persistence, security, and tests
|-- DentalWave-frontend/    Active React manager interface
|-- portable/windows/       Windows launch, shutdown, backup, and diagnostics
|-- portable/tests/         Static Windows launcher/path tests
|-- docs/                   Screenshots and focused architecture/test notes
|-- build-portable.sh       Windows portable distribution builder
`-- README.md
```

## Current limitations and release considerations

- The supported office distribution is currently Windows x64. There is no packaged macOS launcher or bundled macOS runtime yet.
- Portable mode is local to one computer and is not a shared network or cloud database service.
- Hibernate currently manages H2 schema updates; the repository does not include Flyway or Liquibase migrations.
- Reusable team templates are not automatically applied during monthly generation.
- Windows startup, shutdown, backup/recovery, restart persistence, and physical printing should be verified on the target office computer and printer before release.
- No automated end-to-end browser suite or application auto-update mechanism is included.
