// macro_rules! expect_or_err {
//     // Handle Option type
//     ($optional:expr, $error_msg:expr, $error_type:path) => {{
//         match $optional {
//             Some(value) => value,
//             None => {
//                 #[cfg(test)]
//                 unreachable!($error_msg);

//                 #[allow(unreachable_code)]
//                 {
//                     Err($error_type)?
//                 }
//             }
//         }
//     }};
//     // Handle boolean type
//     ($condition:expr, $error_msg:expr, $error_type:path, bool) => {{
//         if !$condition {
//             #[cfg(test)]
//             unreachable!($error_msg);

//             #[allow(unreachable_code)]
//             {
//                 Err($error_type)?
//             }
//         }
//     }};
// }

use crate::{pallet, Pallet};

impl<T> Pallet<T> where T: pallet::Config {}
