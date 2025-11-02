use std::fmt::{Display, Formatter};
use std::str::FromStr;
use strum::{EnumCount, EnumIter};
use thiserror::Error;
use BugParseError::InvalidBugCharacter;

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, Ord, PartialOrd, EnumIter, EnumCount)]
pub enum Bug {
    Ant,
    Beetle,
    Grasshopper,
    Queen,
    Spider,
}

impl Display for Bug {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Bug::Beetle => "B",
                Bug::Queen => "Q",
                Bug::Grasshopper => "G",
                Bug::Ant => "A",
                Bug::Spider => "S",
            }
        )
    }
}

impl FromStr for Bug {
    type Err = BugParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "B" => Ok(Bug::Beetle),
            "Q" => Ok(Bug::Queen),
            "G" => Ok(Bug::Grasshopper),
            "A" => Ok(Bug::Ant),
            "S" => Ok(Bug::Spider),
            _ => Err(InvalidBugCharacter(s.to_string())),
        }
    }
}

#[derive(Error, Debug)]
pub enum BugParseError {
    #[error("Invalid bug character: {0}")]
    InvalidBugCharacter(String),
}