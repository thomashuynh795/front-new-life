use crate::location::domain::{country_enum::Country, thoroughfare_enum::Thoroughfare};

pub struct Address {
    country: Country,
    thoroughfare: Thoroughfare,
    line_1: String,
    line_2: Option<String>,
}

impl Address {
    #[must_use]
    pub const fn new(
        country: Country,
        thoroughfare: Thoroughfare,
        line_1: String,
        line_2: Option<String>,
    ) -> Self {
        Self {
            country,
            thoroughfare,
            line_1,
            line_2,
        }
    }

    #[must_use]
    pub const fn get_country(&self) -> &Country {
        &self.country
    }

    #[must_use]
    pub const fn get_thoroughfare(&self) -> &Thoroughfare {
        &self.thoroughfare
    }

    #[must_use]
    pub fn get_line_1(&self) -> &str {
        &self.line_1
    }

    #[must_use]
    pub fn get_line_2(&self) -> Option<&str> {
        self.line_2.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_address() {
        let address = Address::new(
            Country::France,
            Thoroughfare::Street,
            String::from("1 street of the republic"),
            None,
        );

        assert!(matches!(address.get_country(), Country::France));
        assert!(matches!(address.get_thoroughfare(), Thoroughfare::Street));
        assert!(matches!(address.get_line_1(), "1 street of the republic"));
        assert!(address.get_line_2().is_none());
    }
}
