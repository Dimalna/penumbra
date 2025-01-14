use std::pin::Pin;

use async_stream::try_stream;
use futures::stream::{StreamExt, TryStreamExt};
use penumbra_chain::View as _;
use penumbra_proto::{
    chain::{ChainParams, CompactBlock, KnownAssets},
    client::oblivious::{
        oblivious_query_server::ObliviousQuery, AssetListRequest, ChainParamsRequest,
        CompactBlockRangeRequest, ValidatorInfoRequest,
    },
    stake::ValidatorInfo,
    Protobuf,
};
use penumbra_shielded_pool::View as _;
use penumbra_stake::{component::View as _, validator};
use tonic::Status;
use tracing::{instrument, Instrument};

// TODO(hdevalence): this still doesn't work, giving up for now
// We need to use the tracing-futures version of Instrument,
// because we want to instrument a Stream, and the Stream trait
// isn't in std, and the tracing::Instrument trait only works with
// (stable) std types.
// use tracing_futures::Instrument;

use super::Info;

#[tonic::async_trait]
impl ObliviousQuery for Info {
    type CompactBlockRangeStream =
        Pin<Box<dyn futures::Stream<Item = Result<CompactBlock, tonic::Status>> + Send>>;

    type ValidatorInfoStream =
        Pin<Box<dyn futures::Stream<Item = Result<ValidatorInfo, tonic::Status>> + Send>>;

    #[instrument(skip(self, request))]
    async fn chain_params(
        &self,
        request: tonic::Request<ChainParamsRequest>,
    ) -> Result<tonic::Response<ChainParams>, Status> {
        let state = self.state_tonic().await?;
        state.check_chain_id(&request.get_ref().chain_id).await?;

        let chain_params = state
            .get_chain_params()
            .await
            .map_err(|_| tonic::Status::unavailable("database error"))?;

        Ok(tonic::Response::new(chain_params.into()))
    }

    #[instrument(skip(self, request))]
    async fn asset_list(
        &self,
        request: tonic::Request<AssetListRequest>,
    ) -> Result<tonic::Response<KnownAssets>, Status> {
        let state = self.state_tonic().await?;
        state.check_chain_id(&request.get_ref().chain_id).await?;

        let known_assets = state
            .known_assets()
            .await
            .map_err(|_| tonic::Status::unavailable("database error"))?;
        Ok(tonic::Response::new(known_assets.into()))
    }

    #[instrument(skip(self, request), fields(show_inactive = request.get_ref().show_inactive))]
    async fn validator_info(
        &self,
        request: tonic::Request<ValidatorInfoRequest>,
    ) -> Result<tonic::Response<Self::ValidatorInfoStream>, Status> {
        let state = self.state_tonic().await?;
        state.check_chain_id(&request.get_ref().chain_id).await?;

        let validators = state
            .validator_list()
            .await
            .map_err(|_| tonic::Status::unavailable("database error"))?;

        let show_inactive = request.get_ref().show_inactive;
        let s = try_stream! {
            for identity_key in validators {
                let info = state.validator_info(&identity_key)
                    .await?
                    .expect("known validator must be present");
                // Slashed and inactive validators are not shown by default.
                if !show_inactive && info.status.state != validator::State::Active {
                    continue;
                }
                yield info.to_proto();
            }
        };

        Ok(tonic::Response::new(
            s.map_err(|_: anyhow::Error| tonic::Status::unavailable("database error"))
                // TODO: how do we instrument a Stream
                //.instrument(Span::current())
                .boxed(),
        ))
    }

    #[instrument(
        skip(self, request),
        fields(
            start_height = request.get_ref().start_height,
            end_height = request.get_ref().end_height,
        ),
    )]
    async fn compact_block_range(
        &self,
        request: tonic::Request<CompactBlockRangeRequest>,
    ) -> Result<tonic::Response<Self::CompactBlockRangeStream>, Status> {
        let state = self.state_tonic().await?;
        state.check_chain_id(&request.get_ref().chain_id).await?;

        let CompactBlockRangeRequest {
            start_height,
            end_height,
            ..
        } = request.into_inner();

        let current_height = state
            .get_block_height()
            .await
            .map_err(|_| tonic::Status::unavailable("database error"))?;

        // Treat end_height = 0 as end_height = current_height so that if the
        // end_height is unspecified in the proto, it will be treated as a
        // request to sync up to the current height.
        let end_height = if end_height == 0 {
            current_height
        } else {
            std::cmp::min(end_height, current_height)
        };

        // We want any events that happen to occur in the context of the
        // current span, but the stream macro doesn't play well with Instrument,
        // so instead just patch it up as best as we can
        let span = tracing::Span::current();
        let block_range = try_stream! {
            // It's useful to record the end height since we adjusted it,
            // but the start height is already recorded in the span.
            span.in_scope(||
                tracing::debug!(
                    end_height,
                    num_blocks = end_height.saturating_sub(start_height),
                    "starting compact_block_range response"
                )
            );
            for height in start_height..end_height {
                let block = state.compact_block(height)
                    .instrument(span.clone())
                    .await?
                    .expect("compact block for in-range height must be present");
                yield block.to_proto();
            }
            span.in_scope(|| tracing::debug!("finished compact_block_range response"));
        };

        Ok(tonic::Response::new(
            block_range
                .map_err(|_: anyhow::Error| tonic::Status::unavailable("database error"))
                // TODO: how to instrument a Stream?
                //.instrument(Span::current())
                .boxed(),
        ))
    }
}
