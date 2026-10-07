#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, serde::Serialize, serde::Deserialize)]
struct A;

#[cfg_attr(feature = "serde", derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default))]
struct B;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, VeryLongDeriveMacroName)]
struct C;

#[derive(Debug, Clone)]
struct D;
