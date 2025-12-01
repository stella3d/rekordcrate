// Copyright (c) 2025 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! This library provides access to device libraries exported from Pioneer's Rekordbox DJ software.

#![warn(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(rust_2018_idioms)]
#![deny(rust_2021_compatibility)]
#![deny(missing_debug_implementations)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(clippy::all)]
#![deny(clippy::explicit_deref_methods)]
#![deny(clippy::explicit_into_iter_loop)]
#![deny(clippy::explicit_iter_loop)]
#![deny(clippy::must_use_candidate)]
#![cfg_attr(not(test), deny(clippy::panic_in_result_fn))]
#![cfg_attr(not(debug_assertions), deny(clippy::used_underscore_binding))]

pub mod anlz;
pub mod pdb;
pub mod setting;
pub mod util;
pub mod xml;
pub(crate) mod xor;

use binrw::BinRead;
use std::{path::PathBuf, slice};

use crate::pdb::{DatabaseType, Header, PageContent, Row};
pub use crate::util::RekordcrateError as Error;
pub use crate::util::RekordcrateResult as Result;

/// Reads all data rows from a PDB file and returns an owned collection for borrowed iteration.
pub fn iter_pdb_rows(path: &PathBuf, typ: DatabaseType) -> Result<PdbRows> {
    let mut reader = std::fs::File::open(path)?;
    let header = Header::read_args(&mut reader, (typ,))?;

    let tables_len = header.tables.len();
    println!("PDB header - # of tables: {}, page size: {}", tables_len, header.page_size);

    // estimate capacity to reduce resize costs 
    let mut rows = Vec::with_capacity(tables_len * 128); 
    for table in &header.tables {
        for page in header.read_pages(
            &mut reader,
            binrw::Endian::NATIVE,
            (&table.first_page, &table.last_page, typ),
        )? {
            if let PageContent::Data(data_content) = page.content {
                for row_group in data_content.row_groups {
                    rows.extend(row_group.into_rows());
                }
            }
        }
    }

    println!("PDB read complete.");
    let row_avg = rows.len() as f32 / tables_len as f32;
    println!("total rows read: {}, rows per table average: {}", rows.len(), row_avg);

    Ok(PdbRows { rows })
}

/// Owned collection of rows extracted from a PDB file.
#[derive(Debug, Default)]
pub struct PdbRows {
    rows: Vec<Row>,
}

impl PdbRows {
    /// Returns `true` if no rows were extracted.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Returns the number of extracted rows.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Iterates over the extracted rows by reference.
    #[must_use]
    pub fn iter(&self) -> PdbRowIter<'_> {
        PdbRowIter {
            inner: self.rows.iter(),
        }
    }

    /// Consumes the container and returns the owned rows.
    #[must_use]
    pub fn into_rows(self) -> Vec<Row> {
        self.rows
    }
}

impl<'a> IntoIterator for &'a PdbRows {
    type Item = &'a Row;
    type IntoIter = PdbRowIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator over raw rows extracted from a PDB file.
#[derive(Debug)]
pub struct PdbRowIter<'a> {
    inner: slice::Iter<'a, Row>,
}

impl<'a> Iterator for PdbRowIter<'a> {
    type Item = &'a Row;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a> ExactSizeIterator for PdbRowIter<'a> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<'a> DoubleEndedIterator for PdbRowIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}
