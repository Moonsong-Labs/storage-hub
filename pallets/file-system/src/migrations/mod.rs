// This file is part of StorageHub.

// Copyright (C) Moonsong Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Migrations for the File System pallet.

/// Module containing migration logic from storage version 0 to 1.
///
/// This migration transforms the `msp` field in `StorageRequestMetadata` from
/// `Option<(ProviderId, bool)>` to `MspStorageRequestStatus<T>`.
pub mod v1;




