//! Marker for the Bevy entity controlled by this local client.

use bevy::prelude::Component;

/// Marks the entity controlled by this client.
///
/// The SpacetimeDB bridge inserts it when a replicated row belongs to the
/// connection identity. It is client-local state and is never authoritative.
#[derive(Component, Debug, Clone, Copy)]
pub struct LocalPlayer;
