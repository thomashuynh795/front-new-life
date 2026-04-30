use crate::location::continent_enum::Continent;

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
    pub const fn get_code(&self) -> &str {
        match self {
            Self::France => "FR",
            Self::Germany => "DE",
            Self::Luxembourg => "LU",
            Self::Monaco => "MO",
            Self::Spain => "ES",
            Self::UnitedKingdom => "UK",
        }
    }

    #[must_use]
    pub const fn get_name(&self) -> &str {
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
    pub const fn get_continent(&self) -> Continent {
        match self {
            Self::France
            | Self::Germany
            | Self::Luxembourg
            | Self::Monaco
            | Self::Spain
            | Self::UnitedKingdom => Continent::Europe,
        }
    }
}
