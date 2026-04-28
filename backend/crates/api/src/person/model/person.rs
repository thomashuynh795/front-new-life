use crate::person::person_type_enum::PersonType;

pub struct Person {
    person_type: PersonType,
    first_name: String,
    last_name: String,
}

impl Person {
    pub const fn new(person_type: PersonType, first_name: String, last_name: String) -> Self {
        Self {
            first_name,
            last_name,
            person_type,
        }
    }

    pub const fn person_type(&self) -> &PersonType {
        &self.person_type
    }

    pub fn first_name(&self) -> &str {
        &self.first_name
    }

    pub fn last_name(&self) -> &str {
        &self.last_name
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

        assert!(matches!(person.person_type(), PersonType::NaturalPerson));
        assert_eq!(person.first_name(), "John");
        assert_eq!(person.last_name(), "Doe")
    }
}
