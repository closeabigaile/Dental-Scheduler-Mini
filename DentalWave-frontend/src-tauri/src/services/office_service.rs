use crate::database::Database;
use crate::error::{AppError, AppResult};
use crate::models::{Office, OfficeInput};
use crate::repositories::office_repository::OfficeRepository;

pub struct OfficeService<'a> {
  database: &'a Database,
}

impl<'a> OfficeService<'a> {
  pub fn new(database: &'a Database) -> Self {
    Self { database }
  }

  pub fn list(&self) -> AppResult<Vec<Office>> {
    OfficeRepository::list(&self.database.connect()?)
  }

  pub fn get(&self, id: i64) -> AppResult<Office> {
    Self::validate_id(id)?;
    OfficeRepository::get(&self.database.connect()?, id)
  }

  pub fn create(&self, input: OfficeInput) -> AppResult<Office> {
    let normalized = Self::normalize(input)?;
    OfficeRepository::create(&self.database.connect()?, &normalized)
  }

  pub fn update(&self, id: i64, input: OfficeInput) -> AppResult<Office> {
    Self::validate_id(id)?;
    let normalized = Self::normalize(input)?;
    OfficeRepository::update(&self.database.connect()?, id, &normalized)
  }

  pub fn delete(&self, id: i64) -> AppResult<()> {
    Self::validate_id(id)?;
    OfficeRepository::delete(&self.database.connect()?, id)
  }

  fn normalize(input: OfficeInput) -> AppResult<OfficeInput> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
      return Err(AppError::invalid_input("Office name is required."));
    }
    Ok(OfficeInput {
      name,
      address: input.address.trim().to_string(),
      phone_number: input.phone_number.trim().to_string(),
    })
  }

  fn validate_id(id: i64) -> AppResult<()> {
    if id <= 0 {
      return Err(AppError::invalid_input("A valid office id is required."));
    }
    Ok(())
  }
}
