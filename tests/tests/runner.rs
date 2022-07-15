use graph_tests::fixture::ethereum::{chain, empty_block, genesis};
use graph_tests::fixture::{self, stores, test_ptr};
use std::time::Duration;

use anyhow::anyhow;
use graph::blockchain::{Block, BlockPtr};
use graph::prelude::ethabi::ethereum_types::H256;
use graph::prelude::{SubgraphAssignmentProvider, SubgraphName, SubgraphStore as _};
use slog::{debug, info};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn data_source_revert() -> anyhow::Result<()> {
    let subgraph_name = SubgraphName::new("data-source-revert")
        .expect("Subgraph name must contain only a-z, A-Z, 0-9, '-' and '_'");

    let hash = {
        let test_dir = format!("./integration-tests/{}", subgraph_name);
        fixture::build_subgraph(&test_dir).await
    };

    let blocks = {
        let block_0 = genesis();
        let block_1 = empty_block(block_0.ptr(), test_ptr(1));
        let block_1_reorged_ptr = BlockPtr {
            number: 1,
            hash: H256::from_low_u64_be(12).into(),
        };
        let block_1_reorged = empty_block(block_0.ptr(), block_1_reorged_ptr);
        vec![block_0, block_1, block_1_reorged]
    };

    let stop_block = blocks.last().unwrap().block.ptr();

    let stores = stores("./integration-tests/config.simple.toml").await;
    let chain = chain(blocks, &stores).await;
    let ctx = fixture::setup(subgraph_name.clone(), &hash, &stores, chain).await;

    let provider = ctx.provider.clone();
    let store = ctx.store.clone();

    let logger = ctx.logger_factory.subgraph_logger(&ctx.deployment_locator);

    SubgraphAssignmentProvider::start(provider.as_ref(), ctx.deployment_locator.clone(), None)
        .await
        .expect("unable to start subgraph");

    loop {
        tokio::time::sleep(Duration::from_millis(1000)).await;

        let block_ptr = match store.least_block_ptr(&hash).await {
            Ok(Some(ptr)) => ptr,
            res => {
                info!(&logger, "{:?}", res);
                continue;
            }
        };

        debug!(&logger, "subgraph block: {:?}", block_ptr);

        if block_ptr == stop_block {
            info!(
                &logger,
                "subgraph now at block {}, reached stop block {}", block_ptr.number, stop_block
            );
            break;
        }

        if !store.is_healthy(&hash).await.unwrap() {
            return Err(anyhow!("subgraph failed unexpectedly"));
        }
    }

    assert!(store.is_healthy(&hash).await.unwrap());

    fixture::cleanup(&ctx.store, &subgraph_name, &hash);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn typename() -> anyhow::Result<()> {
    let subgraph_name = SubgraphName::new("typename")
        .expect("Subgraph name must contain only a-z, A-Z, 0-9, '-' and '_'");

    let hash = {
        let test_dir = format!("./integration-tests/{}", subgraph_name);
        fixture::build_subgraph(&test_dir).await
    };

    let chain: Vec<BlockWithTriggers<Chain>> = {
        let block0_hash = H256::from_low_u64_be(0);
        let block0 = BlockWithTriggers {
            block: BlockFinality::Final(Arc::new(LightEthereumBlock {
                hash: Some(block0_hash),
                number: Some(U64::from(0)),
                ..Default::default()
            })),
            trigger_data: vec![EthereumTrigger::Block(
                test_ptr(0),
                EthereumBlockTriggerType::Every,
            )],
        };
        let block1 = BlockWithTriggers::<Chain> {
            block: BlockFinality::Final(Arc::new(LightEthereumBlock {
                hash: Some(H256::from_low_u64_be(1)),
                number: Some(U64::from(1)),
                parent_hash: block0_hash,
                ..Default::default()
            })),
            trigger_data: vec![EthereumTrigger::Block(
                test_ptr(1),
                EthereumBlockTriggerType::Every,
            )],
        };
        let block1_hash_reorged = H256::from_low_u64_be(11);
        let block1_reorged = BlockWithTriggers::<Chain> {
            block: BlockFinality::Final(Arc::new(LightEthereumBlock {
                hash: Some(block1_hash_reorged),
                number: Some(U64::from(1)),
                parent_hash: block0_hash,
                ..Default::default()
            })),
            trigger_data: vec![EthereumTrigger::Block(
                BlockPtr {
                    hash: block1_hash_reorged.into(),
                    number: 1,
                },
                EthereumBlockTriggerType::Every,
            )],
        };
        let block2_hash = H256::from_low_u64_be(2);
        let block2 = BlockWithTriggers::<Chain> {
            block: BlockFinality::Final(Arc::new(LightEthereumBlock {
                hash: Some(block2_hash),
                number: Some(U64::from(2)),
                parent_hash: block1_hash_reorged,
                ..Default::default()
            })),
            trigger_data: vec![EthereumTrigger::Block(
                BlockPtr {
                    hash: block2_hash.into(),
                    number: 2,
                },
                EthereumBlockTriggerType::Every,
            )],
        };
        let block3_hash = H256::from_low_u64_be(3);
        let block3 = BlockWithTriggers::<Chain> {
            block: BlockFinality::Final(Arc::new(LightEthereumBlock {
                hash: Some(block3_hash),
                number: Some(U64::from(3)),
                parent_hash: block2_hash,
                ..Default::default()
            })),
            trigger_data: vec![EthereumTrigger::Block(
                BlockPtr {
                    hash: block3_hash.into(),
                    number: 3,
                },
                EthereumBlockTriggerType::Every,
            )],
        };
        vec![block0, block1, block1_reorged, block2, block3]
    };

    let stop_block = chain.last().unwrap().block.ptr();

    let ctx = fixture::setup(
        subgraph_name.clone(),
        &hash,
        "./integration-tests/config.simple.toml",
        chain,
    )
    .await;

    let provider = ctx.provider.clone();
    let store = ctx.store.clone();

    let logger = ctx.logger_factory.subgraph_logger(&ctx.deployment_locator);

    SubgraphAssignmentProvider::start(provider.as_ref(), ctx.deployment_locator.clone(), None)
        .await
        .expect("unable to start subgraph");

    loop {
        tokio::time::sleep(Duration::from_millis(1000)).await;

        let block_ptr = match store.least_block_ptr(&hash).await {
            Ok(Some(ptr)) => ptr,
            res => {
                info!(&logger, "{:?}", res);
                continue;
            }
        };

        debug!(&logger, "subgraph block: {:?}", block_ptr);

        if block_ptr == stop_block {
            info!(
                &logger,
                "subgraph now at block {}, reached stop block {}", block_ptr.number, stop_block
            );
            break;
        }

        if !store.is_healthy(&hash).await.unwrap() {
            return Err(anyhow!("subgraph failed unexpectedly"));
        }
    }

    assert!(store.is_healthy(&hash).await.unwrap());

    fixture::cleanup(&ctx.store, &subgraph_name, &hash);

    Ok(())
}
