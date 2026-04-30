use crate::person::person_type_enum::PersonType;

pub struct Person {
    legal_person_type: PersonType,
    first_name: String,
    last_name: String,
}

impl Person {
    #[must_use]
    pub const fn new(legal_person_type: PersonType, first_name: String, last_name: String) -> Self {
        Self {
            legal_person_type,
            first_name,
            last_name,
        }
    }

    #[must_use]
    pub const fn get_person_type(&self) -> &PersonType {
        &self.legal_person_type
    }

    #[must_use]
    pub fn get_first_name(&self) -> &str {
        &self.first_name
    }

    #[must_use]
    pub fn get_last_name(&self) -> &str {
        &self.last_name
    }

    #[must_use]
    pub const fn get_legal_person_type(&self) -> &PersonType {
        &self.legal_person_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_person() {
        let person = Person::new(
            PersonType::NaturalPerson,
            String::from("John"),
            String::from("Doe"),
        );

        assert!(matches!(
            person.get_person_type(),
            PersonType::NaturalPerson
        ));
        assert!(matches!(person.get_first_name(), "John"));
        assert!(matches!(person.get_last_name(), "Doe"));
    }
}
