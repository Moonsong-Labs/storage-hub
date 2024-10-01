use bigdecimal::num_bigint::BigUint;
use diesel::prelude::*;

use crate::{schema::paymentstream, DbConnection};

#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = paymentstream)]
pub struct PaymentStream {
    // postgres attributed ID for this payment stream
    pub id: i32,
    // Account ID of the payer
    pub account: String,
    // ID of the payee (msp or bsp)
    pub provider: String,
    // Total amount already paid to this provider
    pub total_amount_paid: BigUint,
}

impl PaymentStream {
    pub async fn create<'a>(
        conn: &mut DbConnection<'a>,
        account: impl Into<String>,
        provider: impl Into<String>,
    ) -> Result<Self, diesel::result::Error> {
        let paymentstream = diesel::insert_into(paymentstream::table)
            .values((
                paymentstream::account.eq(account),
                paymentstream::provider.eq(provider),
            ))
            .get_result(conn)
            .await?;
        Ok(paymentstream)
    }
}
