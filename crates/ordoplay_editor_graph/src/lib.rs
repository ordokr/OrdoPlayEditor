// SPDX-License-Identifier: MIT OR Apache-2.0
//! Node graph framework for `OrdoPlay` Editor.
//!
//! This crate provides a unified graph framework that powers:
//! - Material/Shader graphs
//! - Gameplay graphs (visual scripting)
//! - Animation state machines
//! - VFX graphs
//!
//! ## Architecture
//!
//! The framework is built on a generic graph model with:
//! - Typed input/output ports
//! - Connection validation
//! - Evaluation scheduling
//! - Serialization support

pub mod node;
pub mod port;
pub mod connection;
pub mod graph;
pub mod evaluation;
pub mod ui;
pub mod graphs;

pub use node::{Node, NodeId, NodeType};
pub use port::{Port, PortId, PortType, PortDirection};
pub use connection::{Connection, ConnectionId};
pub use graph::Graph;
