use std::sync::{Arc, RwLock};

use alloy::{
    eips::BlockId,
    primitives::{Address, Bytes, B256, U256},
    providers::{
        network::{Ethereum, EthereumWallet, TransactionBuilder},
        PendingTransactionBuilder, Provider, ProviderBuilder,
    },
    rpc::{
        client::ClientBuilder,
        types::{BlockTransactionsKind, TransactionRequest},
    },
    signers::local::{LocalSignerError, PrivateKeySigner},
    sol_types::{SolCall, SolInterface},
    transports::{
        http::{reqwest, Client, Http},
        RpcError, TransportErrorKind,
    },
};

crate::stack_error! {
    name: EthError,
    stack_name: EthErrorStack,
    error: {},
    wrap: {
        Signer(LocalSignerError),
        Url(url::ParseError),
        Rpc(RpcError<TransportErrorKind>),
        Type(alloy::sol_types::Error),
        Http(reqwest::Error),
    },
    stack: {
        BuildClient(),
        OnTransact(contract: Address, sig: &'static str),
        OnCall(contract: Address, sig: &'static str),
        OnDecodeReturn(contract: Address, sig: &'static str, data: Bytes),
    }
}

impl EthError {
    pub fn revert(&self) -> Option<Bytes> {
        match self.origin() {
            Self::Rpc(RpcError::ErrorResp(payload)) => payload.as_revert_data(),
            _ => None,
        }
    }

    pub fn revert_data<T: SolInterface>(self) -> Result<(T, EthError), EthError> {
        match self.origin() {
            Self::Rpc(RpcError::ErrorResp(payload)) => match payload.as_revert_data() {
                Some(data) => Ok((T::abi_decode(&data, true)?, self)),
                None => Err(self),
            },
            _ => Err(self),
        }
    }
}

#[derive(Clone)]
pub struct MutexEth(Arc<RwLock<Arc<Eth>>>);

impl MutexEth {
    pub fn new(eth: Eth) -> Self {
        Self(Arc::new(RwLock::new(Arc::new(eth))))
    }

    pub fn get(&self) -> Arc<Eth> {
        self.0.read().unwrap().clone()
    }

    pub fn reset_if_error<E>(&self) -> impl FnOnce(E) -> E + '_ {
        |err| {
            let _result = self.reset();
            err
        }
    }

    pub fn reset(&self) -> Result<(), EthError> {
        let conn = Arc::new(self.0.read().unwrap().new_conn()?);
        let mut eth = self.0.write().unwrap();
        *eth = conn;
        Ok(())
    }
}

#[derive(Clone)]
pub struct Eth {
    endpoint: String,
    private_key: Option<String>,
    client: Arc<Box<dyn Provider<Http<Client>>>>,
}

impl Eth {
    pub fn dial(endpoint: &str, private_key: Option<&str>) -> Result<Eth, EthError> {
        Ok(Eth {
            endpoint: endpoint.to_owned(),
            private_key: private_key.map(|n| n.to_owned()).clone(),
            client: Arc::new(Self::dial_conn(endpoint, private_key)?),
        })
    }

    pub fn new_conn(&self) -> Result<Self, EthError> {
        Ok(Self {
            endpoint: self.endpoint.clone(),
            private_key: self.private_key.clone(),
            client: Arc::new(Self::dial_conn(
                self.endpoint.as_str(),
                self.private_key.as_ref().map(|n| n.as_str()),
            )?),
        })
    }

    fn dial_conn(
        endpoint: &str,
        private_key: Option<&str>,
    ) -> Result<Box<dyn Provider<Http<Client>>>, EthError> {
        let url = endpoint.try_into()?;

        let client = reqwest::ClientBuilder::new()
            .build()
            .map_err(EthError::BuildClient())?;
        let transport = alloy::transports::http::Http::with_client(client, url);
        let is_local = transport.guess_local();
        let client = ClientBuilder::default().transport(transport, is_local);
        Ok(match private_key {
            Some(pk) => {
                let signer = pk.parse::<PrivateKeySigner>()?;
                let wallet = EthereumWallet::new(signer);
                let provider = ProviderBuilder::new()
                    .with_recommended_fillers()
                    .wallet(wallet)
                    .on_client(client);
                Box::new(provider)
            }
            None => {
                let provider = ProviderBuilder::new().on_client(client);
                Box::new(provider)
            }
        })
    }

    pub async fn transact<T: SolCall>(
        &self,
        contract: Address,
        call: &T,
    ) -> Result<PendingTransactionBuilder<Http<Client>, Ethereum>, EthError> {
        let tx = TransactionRequest::default().with_call(call).to(contract);
        let result = self
            .client
            .send_transaction(tx)
            .await
            .map_err(EthError::OnTransact(&contract, &T::SIGNATURE))?;
        Ok(result)
    }

    pub async fn call<T: SolCall>(
        &self,
        contract: Address,
        call: &T,
    ) -> Result<T::Return, EthError> {
        let tx = TransactionRequest::default().with_call(call).to(contract);
        let result = self
            .client
            .call(&tx)
            .await
            .map_err(EthError::OnCall(&contract, &T::SIGNATURE))?;
        let result = T::abi_decode_returns(&result, true).map_err(EthError::OnDecodeReturn(
            &contract,
            &T::SIGNATURE,
            &result,
        ))?;
        Ok(result)
    }

    pub async fn select_reference_block(&self) -> Result<(U256, B256), EthError> {
        // corner case:
        //  1. block numbers may not sequential
        //  2. the types.Header.Hash() may not compatible with the chain
        let k = BlockTransactionsKind::Hashes;
        let head = self.client.get_block(BlockId::latest(), k).await?.unwrap();
        let hash = head.header.parent_hash;
        let reference_block = self.client.get_block(hash.into(), k).await?.unwrap();
        let number = reference_block.header.number.unwrap();
        Ok((U256::from_limbs_slice(&[number]), hash))
    }
}
