// Many of the IBC message types are enums, where the number of variants differs
// depending on the build configuration, meaning that the fallthrough case gets
// marked as unreachable only when not building in test configuration.
#![allow(unreachable_patterns)]

mod channel;
mod client;
mod connection;

use anyhow::Result;
use async_trait::async_trait;
use client::Ics2Client;
use penumbra_chain::genesis;
use penumbra_component::Component;
use penumbra_storage::State;
use penumbra_transaction::Transaction;
use tendermint::abci;
use tracing::instrument;

pub struct IBCComponent {
    client: client::Ics2Client,
    connection: connection::ConnectionComponent,
    channel: channel::ICS4Channel,

    enabled: bool,
}

impl IBCComponent {
    #[instrument(name = "ibc", skip(state))]
    pub async fn new(state: State) -> Self {
        let client = Ics2Client::new(state.clone()).await;
        let connection = connection::ConnectionComponent::new(state.clone()).await;
        let channel = channel::ICS4Channel::new(state.clone()).await;

        Self {
            channel,
            client,
            connection,
            enabled: false,
        }
    }
}

#[async_trait]
impl Component for IBCComponent {
    #[instrument(name = "ibc", skip(self, app_state))]
    async fn init_chain(&mut self, app_state: &genesis::AppState) {
        self.enabled = app_state.chain_params.ibc_enabled;

        self.client.init_chain(app_state).await;
        self.connection.init_chain(app_state).await;
        self.channel.init_chain(app_state).await;
    }

    #[instrument(name = "ibc", skip(self, begin_block))]
    async fn begin_block(&mut self, begin_block: &abci::request::BeginBlock) {
        self.client.begin_block(begin_block).await;
        self.connection.begin_block(begin_block).await;
        self.channel.begin_block(begin_block).await;
    }

    #[instrument(name = "ibc", skip(tx))]
    fn check_tx_stateless(tx: &Transaction) -> Result<()> {
        client::Ics2Client::check_tx_stateless(tx)?;
        connection::ConnectionComponent::check_tx_stateless(tx)?;
        channel::ICS4Channel::check_tx_stateless(tx)?;

        Ok(())
    }

    #[instrument(name = "ibc", skip(self, tx))]
    async fn check_tx_stateful(&self, tx: &Transaction) -> Result<()> {
        if tx.ibc_actions().count() > 0 && !self.enabled {
            return Err(anyhow::anyhow!(
                "transaction contains IBC actions, but IBC is not enabled"
            ));
        }

        self.client.check_tx_stateful(tx).await?;
        self.connection.check_tx_stateful(tx).await?;
        self.channel.check_tx_stateful(tx).await?;

        Ok(())
    }

    #[instrument(name = "ibc", skip(self, tx))]
    async fn execute_tx(&mut self, tx: &Transaction) {
        self.client.execute_tx(tx).await;
        self.connection.execute_tx(tx).await;
        self.channel.execute_tx(tx).await;
    }

    #[instrument(name = "ibc", skip(self, end_block))]
    async fn end_block(&mut self, end_block: &abci::request::EndBlock) {
        self.client.end_block(end_block).await;
        self.connection.end_block(end_block).await;
        self.channel.end_block(end_block).await;
    }
}
