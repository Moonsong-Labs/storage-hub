use bigdecimal::BigDecimal;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{schema::paymentstream, DbConnection};

#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = paymentstream)]
pub struct PaymentStream {
    // postgres attributed ID for this payment stream
    pub id: i64,
    // Account ID of the payer
    pub account: String,
    // ID of the payee (msp or bsp)
    pub provider: String,
    // Total amount already paid to this provider from this account for this payment stream
    pub total_amount_paid: BigDecimal,
    // The last tick for which the payment stream has recorded a payment
    pub last_tick_charged: i64,
    // The tick at which the payment actually happened
    pub charged_at_tick: i64,
    // Rate for fixed-rate payment streams (mutually exclusive with amount_provided)
    pub rate: Option<BigDecimal>,
    // Amount provided for dynamic-rate payment streams (mutually exclusive with rate)
    pub amount_provided: Option<BigDecimal>,
}

impl PaymentStream {
    pub async fn get<'a>(
        conn: &mut DbConnection<'a>,
        account: String,
        provider: String,
    ) -> Result<Self, diesel::result::Error> {
        // Looking by a payment stream by provider and the account associated
        let ps = paymentstream::table
            .filter(
                paymentstream::account
                    .eq(account)
                    .and(paymentstream::provider.eq(provider)),
            )
            .first(conn)
            .await?;
        Ok(ps)
    }

    pub async fn update_total_amount<'a>(
        conn: &mut DbConnection<'a>,
        ps_id: i64,
        new_total_amount: BigDecimal,
        last_tick_charged: i64,
        charged_at_tick: i64,
    ) -> Result<(), diesel::result::Error> {
        diesel::update(paymentstream::table)
            .filter(paymentstream::id.eq(ps_id))
            .set((
                paymentstream::total_amount_paid.eq(new_total_amount),
                paymentstream::last_tick_charged.eq(last_tick_charged),
                paymentstream::charged_at_tick.eq(charged_at_tick),
            ))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn create_fixed_rate<'a>(
        conn: &mut DbConnection<'a>,
        account: String,
        provider: String,
        rate: BigDecimal,
    ) -> Result<Self, diesel::result::Error> {
        let ps = diesel::insert_into(paymentstream::table)
            .values((
                paymentstream::account.eq(account),
                paymentstream::provider.eq(provider),
                paymentstream::rate.eq(Some(rate)),
                paymentstream::amount_provided.eq(None::<BigDecimal>),
            ))
            .returning(PaymentStream::as_select())
            .get_result(conn)
            .await?;
        Ok(ps)
    }

    pub async fn create_dynamic_rate<'a>(
        conn: &mut DbConnection<'a>,
        account: String,
        provider: String,
        amount_provided: BigDecimal,
    ) -> Result<Self, diesel::result::Error> {
        let ps = diesel::insert_into(paymentstream::table)
            .values((
                paymentstream::account.eq(account),
                paymentstream::provider.eq(provider),
                paymentstream::rate.eq(None::<BigDecimal>),
                paymentstream::amount_provided.eq(Some(amount_provided)),
            ))
            .returning(PaymentStream::as_select())
            .get_result(conn)
            .await?;
        Ok(ps)
    }

    pub async fn update_fixed_rate<'a>(
        conn: &mut DbConnection<'a>,
        ps_id: i64,
        new_rate: BigDecimal,
    ) -> Result<(), diesel::result::Error> {
        diesel::update(paymentstream::table)
            .filter(paymentstream::id.eq(ps_id))
            .set(paymentstream::rate.eq(Some(new_rate)))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn update_dynamic_rate<'a>(
        conn: &mut DbConnection<'a>,
        ps_id: i64,
        new_amount_provided: BigDecimal,
    ) -> Result<(), diesel::result::Error> {
        diesel::update(paymentstream::table)
            .filter(paymentstream::id.eq(ps_id))
            .set(paymentstream::amount_provided.eq(Some(new_amount_provided)))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn get_all_by_user<'a>(
        conn: &mut DbConnection<'a>,
        account: String,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        let payment_streams = paymentstream::table
            .filter(paymentstream::account.eq(account))
            .load(conn)
            .await?;
        Ok(payment_streams)
    }
}
