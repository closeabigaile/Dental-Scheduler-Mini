use crate::database::Database;
use crate::error::{AppError, AppResult};
use crate::models::{ResourceInput, ResourceKind, SchedulingResource};
use crate::repositories::office_repository::OfficeRepository;
use crate::repositories::resource_repository::ResourceRepository;
use std::collections::BTreeSet;

pub struct ResourceService<'a> {
  database: &'a Database,
}

impl<'a> ResourceService<'a> {
  pub fn new(database: &'a Database) -> Self {
    Self { database }
  }

  pub fn list(&self, kind: ResourceKind) -> AppResult<Vec<SchedulingResource>> {
    ResourceRepository::list(&self.database.connect()?, kind)
  }

  pub fn get(&self, kind: ResourceKind, id: i64) -> AppResult<SchedulingResource> {
    Self::validate_id(kind, id)?;
    ResourceRepository::get(&self.database.connect()?, kind, id)
  }

  pub fn create(
    &self,
    kind: ResourceKind,
    input: ResourceInput,
  ) -> AppResult<SchedulingResource> {
    let mut connection = self.database.connect()?;
    let normalized = Self::normalize_and_validate(&connection, kind, input)?;
    ResourceRepository::create(&mut connection, kind, &normalized)
  }

  pub fn update(
    &self,
    kind: ResourceKind,
    id: i64,
    input: ResourceInput,
  ) -> AppResult<SchedulingResource> {
    Self::validate_id(kind, id)?;
    let mut connection = self.database.connect()?;
    let normalized = Self::normalize_and_validate(&connection, kind, input)?;
    ResourceRepository::update(&mut connection, kind, id, &normalized)
  }

  pub fn delete(&self, kind: ResourceKind, id: i64) -> AppResult<()> {
    Self::validate_id(kind, id)?;
    ResourceRepository::delete(&self.database.connect()?, kind, id)
  }

  fn normalize_and_validate(
    connection: &rusqlite::Connection,
    kind: ResourceKind,
    mut input: ResourceInput,
  ) -> AppResult<ResourceInput> {
    if let Some(resource_type) = &input.resource_type {
      if resource_type != kind.as_str() {
        return Err(AppError::invalid_input(format!(
          "Resource type must be {}.",
          kind.as_str()
        )));
      }
    }

    input.display_name = input.display_name.trim().to_string();
    if input.display_name.is_empty() {
      return Err(AppError::invalid_input(format!(
        "{} name is required.",
        kind.entity_name()
      )));
    }
    if input.notes.len() > 2000 {
      return Err(AppError::invalid_input("Notes cannot exceed 2,000 characters."));
    }
    input.first_name = input.first_name.trim().to_string();
    if input.first_name.is_empty() {
      input.first_name = input.display_name.clone();
    }
    input.last_name = input.last_name.trim().to_string();
    input.color = input.color.trim().to_string();
    if input.color.is_empty() {
      input.color = "#65a9b8".to_string();
    }
    input.notes = input.notes.trim().to_string();

    let office_ids = input
      .offices
      .iter()
      .map(|office| office.id)
      .chain(input.default_office.iter().map(|office| office.id))
      .collect::<BTreeSet<_>>();
    for office_id in office_ids {
      if office_id <= 0 || !OfficeRepository::exists(connection, office_id)? {
        return Err(AppError::invalid_input(
          "Every selected office must exist in the desktop database.",
        ));
      }
    }
    Ok(input)
  }

  fn validate_id(kind: ResourceKind, id: i64) -> AppResult<()> {
    if id <= 0 {
      return Err(AppError::invalid_input(format!(
        "A valid {} id is required.",
        kind.entity_name().to_lowercase()
      )));
    }
    Ok(())
  }
}
