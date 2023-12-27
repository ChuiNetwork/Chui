//! This module contains the implementation of the Bloom filter data structure.
//!
//! The Bloom filter is a probabilistic data structure used to test whether an element is a member of a set.
//! It provides a space-efficient way to represent a large set of elements and perform membership tests.
//!
//! The `bloom` module provides functions and types for creating, manipulating, and querying Bloom filters.
//!
//! # Examples
//!
//! ```
//! use bloom::BloomFilter;
//!
//! // Create a new Bloom filter with a capacity of 100 elements and a false positive rate of 0.1%
//! let mut filter = BloomFilter::new(100, 0.001);
//!
//! // Insert some elements into the filter
//! filter.insert("apple");
//! filter.insert("banana");
//!
//! // Check if an element is in the filter
//! assert!(filter.contains("apple"));
//! assert!(!filter.contains("orange"));
//! ```

#![cfg_attr(RUSTC_WITH_SPECIALIZATION, feature(min_specialization))]
pub mod bloom;

#[macro_use]
extern crate chui_frozen_abi_macro;
