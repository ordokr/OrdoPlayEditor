// SPDX-License-Identifier: MIT OR Apache-2.0
//! Undo/redo history system using Copy-on-Write patterns.
//!
//! This is a standalone implementation of the undo/redo system
//! based on the patterns from ordoplay_editor.


use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Maximum undo history depth
const MAX_HISTORY: usize = 100;

/// History errors
#[derive(Debug, Error)]
pub enum HistoryError {
    /// Nothing to undo
    #[error("Nothing to undo")]
    NothingToUndo,

    /// Nothing to redo
    #[error("Nothing to redo")]
    NothingToRedo,

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
}

/// Result type for history operations
pub type Result<T> = std::result::Result<T, HistoryError>;

/// Unique operation ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OperationID(u64);

#[allow(dead_code)] // Intentionally kept for API completeness
impl OperationID {
    /// Create a new operation ID
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    pub fn value(&self) -> u64 {
        self.0
    }
}

/// Component state snapshot (CoW data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Serialized component state
    pub data: Vec<u8>,
    /// Timestamp when snapshot was taken
    pub timestamp: u64,
    /// Size in bytes
    pub size: usize,
}

impl StateSnapshot {
    /// Create a new state snapshot
    pub fn new(data: Vec<u8>) -> Self {
        let size = data.len();
        Self {
            data,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            size,
        }
    }

    /// Create from serializable value
    pub fn from_value<T: Serialize>(value: &T) -> Result<Self> {
        let data = bincode::serialize(value)?;
        Ok(Self::new(data))
    }

    /// Deserialize to value
    pub fn to_value<T: for<'de> Deserialize<'de>>(&self) -> Result<T> {
        Ok(bincode::deserialize(&self.data)?)
    }
}

/// An operation that can be undone/redone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    /// Unique operation ID
    pub id: OperationID,
    /// Human-readable description
    pub description: String,
    /// State before operation (for undo)
    pub before: StateSnapshot,
    /// State after operation (for redo)
    pub after: StateSnapshot,
    /// Timestamp
    pub timestamp: u64,
}

impl Operation {
    /// Create a new operation
    pub fn new(
        id: OperationID,
        description: String,
        before: StateSnapshot,
        after: StateSnapshot,
    ) -> Self {
        Self {
            id,
            description,
            before,
            after,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Get memory size of this operation
    pub fn memory_size(&self) -> usize {
        self.before.size + self.after.size
    }
}

/// Group of operations that are undone/redone together
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationGroup {
    /// Group ID
    pub id: OperationID,
    /// Human-readable description
    pub description: String,
    /// Operations in this group
    pub operations: Vec<Operation>,
    /// Timestamp
    pub timestamp: u64,
}

impl OperationGroup {
    /// Create a new operation group
    pub fn new(id: OperationID, description: String) -> Self {
        Self {
            id,
            description,
            operations: Vec::new(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Add an operation to this group
    pub fn add_operation(&mut self, operation: Operation) {
        self.operations.push(operation);
    }

    /// Get total memory size of this group
    pub fn memory_size(&self) -> usize {
        self.operations.iter().map(|op| op.memory_size()).sum()
    }

    /// Get operation count
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn count(&self) -> usize {
        self.operations.len()
    }
}

/// History statistics
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistoryStats {
    /// Total operations in undo stack
    pub undo_count: usize,
    /// Total operations in redo stack
    pub redo_count: usize,
    /// Total memory used by history (bytes)
    pub memory_used: usize,
    /// Maximum history depth
    pub max_depth: usize,
}

/// Undo/redo history manager
#[derive(Debug)]
pub struct History {
    /// Undo stack
    undo_stack: VecDeque<OperationGroup>,
    /// Redo stack
    redo_stack: VecDeque<OperationGroup>,
    /// Next operation ID
    next_id: u64,
    /// Maximum history depth
    max_depth: usize,
    /// Total memory used
    memory_used: usize,
}

impl History {
    /// Create a new history manager
    pub fn new() -> Self {
        Self::with_max_depth(MAX_HISTORY)
    }

    /// Create with custom maximum depth
    pub fn with_max_depth(max_depth: usize) -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            next_id: 1,
            max_depth,
            memory_used: 0,
        }
    }

    /// Begin a new operation
    pub fn begin_operation(&mut self, _description: &str) -> OperationID {
        let id = OperationID(self.next_id);
        self.next_id += 1;
        id
    }

    /// Commit an operation group
    pub fn commit(&mut self, group: OperationGroup) -> Result<()> {
        if group.operations.is_empty() {
            return Ok(());
        }

        // Clear redo stack
        self.redo_stack.clear();

        // Add to undo stack
        self.memory_used += group.memory_size();
        self.undo_stack.push_back(group);

        // Enforce history limit
        while self.undo_stack.len() > self.max_depth {
            if let Some(old_group) = self.undo_stack.pop_front() {
                self.memory_used = self.memory_used.saturating_sub(old_group.memory_size());
            }
        }

        Ok(())
    }

    /// Undo the last operation
    pub fn undo(&mut self) -> Result<OperationGroup> {
        let group = self
            .undo_stack
            .pop_back()
            .ok_or(HistoryError::NothingToUndo)?;

        self.memory_used = self.memory_used.saturating_sub(group.memory_size());
        self.redo_stack.push_back(group.clone());

        Ok(group)
    }

    /// Redo the last undone operation
    pub fn redo(&mut self) -> Result<OperationGroup> {
        let group = self
            .redo_stack
            .pop_back()
            .ok_or(HistoryError::NothingToRedo)?;

        self.memory_used += group.memory_size();
        self.undo_stack.push_back(group.clone());

        Ok(group)
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get undo stack depth
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get redo stack depth
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn redo_depth(&self) -> usize {
        self.redo_stack.len()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.memory_used = 0;
    }

    /// Get history statistics
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn stats(&self) -> HistoryStats {
        HistoryStats {
            undo_count: self.undo_stack.len(),
            redo_count: self.redo_stack.len(),
            memory_used: self.memory_used,
            max_depth: self.max_depth,
        }
    }

    /// Get description of next undo operation
    pub fn undo_description(&self) -> Option<&str> {
        self.undo_stack.back().map(|g| g.description.as_str())
    }

    /// Get description of next redo operation
    pub fn redo_description(&self) -> Option<&str> {
        self.redo_stack.back().map(|g| g.description.as_str())
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}
