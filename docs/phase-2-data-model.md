# Phase 2 desktop data model

This document records the source-model review completed before the Phase 2
SQLite schema was implemented. The existing Spring/H2 application remains the
reference system; this document describes only the smaller desktop scheduler
scope.

## Offices

The Spring `Office` entity and `OfficeDto` contain `id`, `name`, `address`, and
`phoneNumber`. Offices are referenced by scheduling resources and doctor work
rules.

- **Used by Mini Scheduler:** identifier, display name, address, phone number,
  and relationships to doctors/assistants and doctor work patterns.
- **Not present in the source model:** an office active/status flag. Phase 2
  therefore preserves the current hard-delete behavior instead of inventing
  deactivation semantics.
- **Deferred/unclear:** none of the four current fields.

## Doctors

Mini Scheduler doctors are `SchedulingResource` records with type `DOCTOR`;
they are not account-backed `Employee` records. The resource model contains
`id`, `firstName`, `lastName`, `displayName`, `active`, `defaultOffice`,
`offices`, `normalWorkdays`, `color`, and `notes`. Doctor work patterns are
stored separately as `DoctorWorkRule` records.

- **Used by Mini Scheduler:** identifier, display/first/last names, active
  status, default office, office membership, color, notes, and the work-rule
  fields needed to preserve weekly/rotating patterns.
- **Inherited from full DentalWave and excluded:** username, password, email,
  phone number, security role, hire date, PTO balance, responsibilities,
  employee availability, and time-off requests. Those belong to the separate
  account-backed `Employee`/`User` HR model.
- **Deferred/unclear:** `normalWorkdays` remains nullable for compatibility,
  but current React screens use structured doctor work rules instead. Phase 2
  stores those rules but does not port schedule resolution or generation.

## Assistants

Mini Scheduler assistants are `SchedulingResource` records with type
`ASSISTANT`, using the same basic resource fields as doctors.

- **Used by Mini Scheduler:** identifier, display/first/last names, active
  status, default office, office membership, color, and notes.
- **Inherited from full DentalWave and excluded:** all account, authentication,
  HR, PTO, availability, and time-off fields from `Employee`/`User`.
- **Deferred/unclear:** `normalWorkdays` is retained as nullable compatibility
  data but is not interpreted in Phase 2.

## Phase 2 schema decision

SQLite uses separate `doctors` and `assistants` tables, office join tables, and
minimal rotation/work-rule tables so current doctor patterns can be preserved
without implementing the scheduling engine. No H2 records are copied or read,
and no production seed data is included.
