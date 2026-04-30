use crate::{location::domain::models::address::Address, person::person_type_enum::PersonType};

pub struct Person {
    legal_person_type: PersonType,
    first_name: String,
    last_name: String,
    address: Address,
}

impl Person {
    #[must_use]
    pub const fn new(
        legal_person_type: PersonType,
        first_name: String,
        last_name: String,
        address: Address,
    ) -> Self {
        Self {
            legal_person_type,
            first_name,
            last_name,
            address,
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

    #[must_use]
    pub const fn get_address(&self) -> &Address {
        &self.address
    }
}

#[cfg(test)]
mod tests {
    use crate::location::domain::{country_enum::Country, thoroughfare_enum::Thoroughfare};

    use super::*;

    #[test]
    fn create_person() {
        let person = Person::new(
            PersonType::NaturalPerson,
            String::from("John"),
            String::from("Doe"),
            Address::new(
                Country::France,
                Thoroughfare::Street,
                String::from("1 Street of the Republic"),
                None,
            ),
        );

        assert!(matches!(
            person.get_person_type(),
            PersonType::NaturalPerson
        ));
        assert!(matches!(person.get_first_name(), "John"));
        assert!(matches!(person.get_last_name(), "Doe"));
        assert!(matches!(
            person.get_address().get_country(),
            Country::France
        ));
        assert!(matches!(
            person.get_address().get_line_1(),
            "1 Street of the Republic"
        ));
    }
}
