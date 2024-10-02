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
    // Total amount already paid to this provider from this account for this payment stream
    pub total_amount_paid: BigUint,
}

impl PaymentStream {
    pub async fn create<'a>(
        conn: &mut DbConnection<'a>,
        account: impl Into<String>,
        provider: impl Into<String>,
    ) -> Result<Self, diesel::result::Error> {
        let ps = diesel::insert_into(paymentstream::table)
            .values((
                paymentstream::account.eq(account),
                paymentstream::provider.eq(provider),
            ))
            .get_result(conn)
            .await?;
        Ok(ps)
    }

    pub async fn get<'a>(
        conn: &mut DbConnection<'a>,
        account: String,
        provider: String,
    ) -> Result<Self, diesel::result::Error> {
        // Looking by a payment stream by provider and the account associated
        let ps = paymentstream::table
            .filter((
                paymentstream::account.eq(account),
                paymentstream::provider.eq(provider),
            ))
            .first(conn)
            .await?;
        Ok(ps)
    }

    pub async fn update_total_amount<'a>(
        conn: &mut DbConnection<'a>,
        ps_id: i32,
        new_total_amount: BigUint,
    ) -> Result<Self, diesel::result::Error> {
        let ps = diesel::update(paymentstream::table)
            .filter(paymentstream::id.eq(ps_id))
            .set(paymentstream::total_amount_paid.eq(new_total_amount))
            .execute(conn)
            .await?;
        Ok(ps)
    }
}
