use crate::location::region_enum::Region;

pub enum Country {
    France,
    Germany,
    Luxembourg,
    Monaco,
    Spain,
    UnitedKingdom,
}

impl Country {
    #[must_use]
    pub const fn get_iso_3166_alpha_2_code(&self) -> &str {
        match self {
            Self::France => "FR",
            Self::Germany => "DE",
            Self::Luxembourg => "LU",
            Self::Monaco => "MC",
            Self::Spain => "ES",
            Self::UnitedKingdom => "GB",
        }
    }

    #[must_use]
    pub const fn get_official_name(&self) -> &str {
        match self {
            Self::France => "France",
            Self::Germany => "Germany",
            Self::Luxembourg => "Luxembourg",
            Self::Monaco => "Monaco",
            Self::Spain => "Spain",
            Self::UnitedKingdom => "United Kingdom of Great Britain and Northern Ireland",
        }
    }

    #[must_use]
    pub const fn get_common_name(&self) -> &str {
        match self {
            Self::France => "France",
            Self::Germany => "Germany",
            Self::Luxembourg => "Luxembourg",
            Self::Monaco => "Monaco",
            Self::Spain => "Spain",
            Self::UnitedKingdom => "United Kingdom",
        }
    }

    #[must_use]
    pub const fn get_region(&self) -> Region {
        match self {
            Self::France
            | Self::Germany
            | Self::Luxembourg
            | Self::Monaco
            | Self::Spain
            | Self::UnitedKingdom => Region::Europe,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_return_country_code() {
        assert_eq!(Country::France.get_iso_3166_alpha_2_code(), "FR");
        assert_eq!(Country::Germany.get_iso_3166_alpha_2_code(), "DE");
        assert_eq!(Country::Luxembourg.get_iso_3166_alpha_2_code(), "LU");
        assert_eq!(Country::Monaco.get_iso_3166_alpha_2_code(), "MC");
        assert_eq!(Country::Spain.get_iso_3166_alpha_2_code(), "ES");
        assert_eq!(Country::UnitedKingdom.get_iso_3166_alpha_2_code(), "GB");
    }

    #[test]
    fn should_return_country_name() {
        assert_eq!(Country::France.get_official_name(), "France");
        assert_eq!(Country::Germany.get_official_name(), "Germany");
        assert_eq!(Country::Luxembourg.get_official_name(), "Luxembourg");
        assert_eq!(Country::Monaco.get_official_name(), "Monaco");
        assert_eq!(Country::Spain.get_official_name(), "Spain");
        assert_eq!(
            Country::UnitedKingdom.get_official_name(),
            "United Kingdom of Great Britain and Northern Ireland"
        );
    }

    #[test]
    fn should_return_country_continent() {
        assert!(matches!(Country::France.get_region(), Region::Europe));
        assert!(matches!(Country::Germany.get_region(), Region::Europe));
        assert!(matches!(Country::Luxembourg.get_region(), Region::Europe));
        assert!(matches!(Country::Monaco.get_region(), Region::Europe));
        assert!(matches!(Country::Spain.get_region(), Region::Europe));
        assert!(matches!(
            Country::UnitedKingdom.get_region(),
            Region::Europe
        ));
    }
}
