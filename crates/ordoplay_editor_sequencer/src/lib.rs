// SPDX-License-Identifier: MIT OR Apache-2.0
//! Timeline/sequencer for OrdoPlay Editor.
//!
//! This crate provides cinematic and gameplay sequencing:
//! - Transform animation tracks
//! - Property animation tracks
//! - Event tracks
//! - Audio tracks
//! - Camera tracks
//!
//! ## Architecture
//!
//! The sequencer is built on:
//! - Track system with typed keyframes
//! - Entity binding for target objects
//! - Curve interpolation
//! - Playback control

pub mod track;
pub mod keyframe;
pub mod binding;
pub mod sequence;
pub mod ui;

pub use track::{
    Track, TrackId, TrackType,
    TransformTrack, AudioTrack, AudioClip, CameraTrack, CameraCut, CameraBlendType,
    EventTrack, EventMarker,
};
pub use keyframe::{Keyframe, KeyframeId, InterpolationMode, KeyframeValue, Interpolation};
pub use binding::{EntityBinding, EntityId};
pub use sequence::{Sequence, SequenceId, PlaybackState, PlaybackController, LoopMarker, TimeMarker};
pub use ui::{SequencerState, SequencerPanel, ViewMode, Selection, DragOperation};
