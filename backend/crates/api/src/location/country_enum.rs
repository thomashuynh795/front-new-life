pub enum Country {
    France,
    Germany,
    Luxembourg,
    Spain,
}

impl Country {
    #[must_use]
    pub const fn code(&self) -> &str {
        match self {
            Self::France => "FR",
            Self::Germany => "DE",
            Self::Luxembourg => "LU",
            Self::Spain => "ES",
        }
    }

    #[must_use]
    pub const fn name(&self) -> &str {
        match self {
            Self::France => "France",
            Self::Germany => "Germany",
            Self::Luxembourg => "Luxembourg",
            Self::Spain => "Spain",
        }
    }
}
