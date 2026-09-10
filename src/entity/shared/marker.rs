/// Implement on the canonical command struct, not its `*Query` read sibling.
pub trait Entity {
    const NAME: &'static str;
}
