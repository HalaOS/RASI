pub mod tuples {
    #[derive(serde :: Serialize, serde :: Deserialize)]
    pub struct IMakerMetadata {
        pub sku: reweb3::runtimes::Address,
        pub sku_quantity_or_id: reweb3::runtimes::U256,
        pub payment_currency: reweb3::runtimes::Address,
        pub price_quantity_or_id: reweb3::runtimes::U256,
        pub sku_type: reweb3::runtimes::U128,
        pub payment_currency_type: reweb3::runtimes::U128,
    }
}
pub mod events {
    pub struct Delist {
        pub from: reweb3::runtimes::Address,
        pub token_id: reweb3::runtimes::U256,
        pub to: reweb3::runtimes::Address,
    }
    impl Delist {
        pub fn signature() -> &'static str {
            "Delist(address,uint256,address)"
        }
    }
    impl Delist {
        pub fn from_log(log: reweb3::runtimes::Log) -> std::io::Result<Self> {
            if log.topics.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            if reweb3::runtimes::keccak256(Self::signature()) != log.topics[0] {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: signature mismatch"),
                ));
            }
            if !(log.topics.len() > 1usize) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            let from: reweb3::runtimes::Address = reweb3::runtimes::from_abi(&log.topics[1usize])
                .map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
            })?;
            if !(log.topics.len() > 2usize) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            let token_id: reweb3::runtimes::U256 = reweb3::runtimes::from_abi(&log.topics[2usize])
                .map_err(|err| {
                    std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
                })?;
            if !(log.topics.len() > 3usize) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            let to: reweb3::runtimes::Address = reweb3::runtimes::from_abi(&log.topics[3usize])
                .map_err(|err| {
                    std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
                })?;
            let (): () = reweb3::runtimes::from_abi(log.data).map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
            })?;
            Ok(Delist { from, token_id, to })
        }
    }
    pub struct List {
        pub from: reweb3::runtimes::Address,
        pub token_id: reweb3::runtimes::U256,
        pub to: reweb3::runtimes::Address,
    }
    impl List {
        pub fn signature() -> &'static str {
            "List(address,uint256,address)"
        }
    }
    impl List {
        pub fn from_log(log: reweb3::runtimes::Log) -> std::io::Result<Self> {
            if log.topics.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            if reweb3::runtimes::keccak256(Self::signature()) != log.topics[0] {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: signature mismatch"),
                ));
            }
            if !(log.topics.len() > 1usize) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            let from: reweb3::runtimes::Address = reweb3::runtimes::from_abi(&log.topics[1usize])
                .map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
            })?;
            if !(log.topics.len() > 2usize) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            let token_id: reweb3::runtimes::U256 = reweb3::runtimes::from_abi(&log.topics[2usize])
                .map_err(|err| {
                    std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
                })?;
            if !(log.topics.len() > 3usize) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            let to: reweb3::runtimes::Address = reweb3::runtimes::from_abi(&log.topics[3usize])
                .map_err(|err| {
                    std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
                })?;
            let (): () = reweb3::runtimes::from_abi(log.data).map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
            })?;
            Ok(List { from, token_id, to })
        }
    }
    pub struct MakerBurn {
        pub from: reweb3::runtimes::Address,
        pub token_id: reweb3::runtimes::U256,
    }
    impl MakerBurn {
        pub fn signature() -> &'static str {
            "MakerBurn(address,uint256)"
        }
    }
    impl MakerBurn {
        pub fn from_log(log: reweb3::runtimes::Log) -> std::io::Result<Self> {
            if log.topics.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            if reweb3::runtimes::keccak256(Self::signature()) != log.topics[0] {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: signature mismatch"),
                ));
            }
            if !(log.topics.len() > 1usize) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            let from: reweb3::runtimes::Address = reweb3::runtimes::from_abi(&log.topics[1usize])
                .map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
            })?;
            if !(log.topics.len() > 2usize) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            let token_id: reweb3::runtimes::U256 = reweb3::runtimes::from_abi(&log.topics[2usize])
                .map_err(|err| {
                    std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
                })?;
            let (): () = reweb3::runtimes::from_abi(log.data).map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
            })?;
            Ok(MakerBurn { from, token_id })
        }
    }
    pub struct MakerMint {
        pub from: reweb3::runtimes::Address,
        pub token_id: reweb3::runtimes::U256,
        pub dex: reweb3::runtimes::Address,
    }
    impl MakerMint {
        pub fn signature() -> &'static str {
            "MakerMint(address,uint256,address)"
        }
    }
    impl MakerMint {
        pub fn from_log(log: reweb3::runtimes::Log) -> std::io::Result<Self> {
            if log.topics.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            if reweb3::runtimes::keccak256(Self::signature()) != log.topics[0] {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: signature mismatch"),
                ));
            }
            if !(log.topics.len() > 1usize) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            let from: reweb3::runtimes::Address = reweb3::runtimes::from_abi(&log.topics[1usize])
                .map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
            })?;
            if !(log.topics.len() > 2usize) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            let token_id: reweb3::runtimes::U256 = reweb3::runtimes::from_abi(&log.topics[2usize])
                .map_err(|err| {
                    std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
                })?;
            if !(log.topics.len() > 3usize) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            let dex: reweb3::runtimes::Address = reweb3::runtimes::from_abi(&log.topics[3usize])
                .map_err(|err| {
                    std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
                })?;
            let (): () = reweb3::runtimes::from_abi(log.data).map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
            })?;
            Ok(MakerMint {
                from,
                token_id,
                dex,
            })
        }
    }
    pub struct MakerUpdate {
        pub from: reweb3::runtimes::Address,
        pub token_id: reweb3::runtimes::U256,
        pub settle_sku_quantity_or_id: reweb3::runtimes::U256,
        pub settle_payment_or_id: reweb3::runtimes::U256,
    }
    impl MakerUpdate {
        pub fn signature() -> &'static str {
            "MakerUpdate(address,uint256,uint256,uint256)"
        }
    }
    impl MakerUpdate {
        pub fn from_log(log: reweb3::runtimes::Log) -> std::io::Result<Self> {
            if log.topics.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            if reweb3::runtimes::keccak256(Self::signature()) != log.topics[0] {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: signature mismatch"),
                ));
            }
            if !(log.topics.len() > 1usize) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            let from: reweb3::runtimes::Address = reweb3::runtimes::from_abi(&log.topics[1usize])
                .map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
            })?;
            if !(log.topics.len() > 2usize) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            let token_id: reweb3::runtimes::U256 = reweb3::runtimes::from_abi(&log.topics[2usize])
                .map_err(|err| {
                    std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
                })?;
            let (settle_sku_quantity_or_id, settle_payment_or_id): (
                reweb3::runtimes::U256,
                reweb3::runtimes::U256,
            ) = reweb3::runtimes::from_abi(log.data).map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
            })?;
            Ok(MakerUpdate {
                from,
                token_id,
                settle_sku_quantity_or_id,
                settle_payment_or_id,
            })
        }
    }
    pub struct OwnershipTransferred {
        pub previous_owner: reweb3::runtimes::Address,
        pub new_owner: reweb3::runtimes::Address,
    }
    impl OwnershipTransferred {
        pub fn signature() -> &'static str {
            "OwnershipTransferred(address,address)"
        }
    }
    impl OwnershipTransferred {
        pub fn from_log(log: reweb3::runtimes::Log) -> std::io::Result<Self> {
            if log.topics.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            if reweb3::runtimes::keccak256(Self::signature()) != log.topics[0] {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: signature mismatch"),
                ));
            }
            if !(log.topics.len() > 1usize) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            let previous_owner: reweb3::runtimes::Address =
                reweb3::runtimes::from_abi(&log.topics[1usize]).map_err(|err| {
                    std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
                })?;
            if !(log.topics.len() > 2usize) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            let new_owner: reweb3::runtimes::Address =
                reweb3::runtimes::from_abi(&log.topics[2usize]).map_err(|err| {
                    std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
                })?;
            let (): () = reweb3::runtimes::from_abi(log.data).map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
            })?;
            Ok(OwnershipTransferred {
                previous_owner,
                new_owner,
            })
        }
    }
    pub struct TakerBurn {
        pub from: reweb3::runtimes::Address,
        pub token_id: reweb3::runtimes::U256,
        pub response_sku_quantity_or_id: reweb3::runtimes::U256,
        pub response_price_quantity_or_id: reweb3::runtimes::U256,
    }
    impl TakerBurn {
        pub fn signature() -> &'static str {
            "TakerBurn(address,uint256,uint256,uint256)"
        }
    }
    impl TakerBurn {
        pub fn from_log(log: reweb3::runtimes::Log) -> std::io::Result<Self> {
            if log.topics.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            if reweb3::runtimes::keccak256(Self::signature()) != log.topics[0] {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: signature mismatch"),
                ));
            }
            if !(log.topics.len() > 1usize) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            let from: reweb3::runtimes::Address = reweb3::runtimes::from_abi(&log.topics[1usize])
                .map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
            })?;
            if !(log.topics.len() > 2usize) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            let token_id: reweb3::runtimes::U256 = reweb3::runtimes::from_abi(&log.topics[2usize])
                .map_err(|err| {
                    std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
                })?;
            let (response_sku_quantity_or_id, response_price_quantity_or_id): (
                reweb3::runtimes::U256,
                reweb3::runtimes::U256,
            ) = reweb3::runtimes::from_abi(log.data).map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
            })?;
            Ok(TakerBurn {
                from,
                token_id,
                response_sku_quantity_or_id,
                response_price_quantity_or_id,
            })
        }
    }
    pub struct TakerMint {
        pub from: reweb3::runtimes::Address,
        pub token_id: reweb3::runtimes::U256,
        pub request_sku_quantity_or_id: reweb3::runtimes::U256,
        pub request_price_quantity_or_id: reweb3::runtimes::U256,
    }
    impl TakerMint {
        pub fn signature() -> &'static str {
            "TakerMint(address,uint256,uint256,uint256)"
        }
    }
    impl TakerMint {
        pub fn from_log(log: reweb3::runtimes::Log) -> std::io::Result<Self> {
            if log.topics.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            if reweb3::runtimes::keccak256(Self::signature()) != log.topics[0] {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: signature mismatch"),
                ));
            }
            if !(log.topics.len() > 1usize) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            let from: reweb3::runtimes::Address = reweb3::runtimes::from_abi(&log.topics[1usize])
                .map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
            })?;
            if !(log.topics.len() > 2usize) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("decode log failed: topic out of range"),
                ));
            }
            let token_id: reweb3::runtimes::U256 = reweb3::runtimes::from_abi(&log.topics[2usize])
                .map_err(|err| {
                    std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
                })?;
            let (request_sku_quantity_or_id, request_price_quantity_or_id): (
                reweb3::runtimes::U256,
                reweb3::runtimes::U256,
            ) = reweb3::runtimes::from_abi(log.data).map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err))
            })?;
            Ok(TakerMint {
                from,
                token_id,
                request_sku_quantity_or_id,
                request_price_quantity_or_id,
            })
        }
    }
}
pub mod errors {}
pub struct PersonalWallet<C> {
    address: reweb3::runtimes::Address,
    client: C,
}
impl<C> PersonalWallet<C> {
    pub fn new(client: C, address: reweb3::runtimes::Address) -> Self {
        Self { address, client }
    }
}
impl<C> PersonalWallet<C> {
    #[doc = r" Deploy contract with provided client."]
    pub async fn deploy<Weth, Ops>(
        client: C,
        weth: Weth,
        transfer_ops: Ops,
    ) -> std::io::Result<Self>
    where
        C: reweb3::runtimes::SignerWithProvider + Send + Unpin,
        Ops: TryInto<reweb3::runtimes::TransferOptions>,
        Ops::Error: std::fmt::Debug + Send + 'static,
        Weth: TryInto<reweb3::runtimes::Address>,
        Weth::Error: std::fmt::Debug + Send + 'static,
    {
        let transfer_ops = transfer_ops.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let weth = weth.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let params = reweb3::runtimes::to_abi((weth,)).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let address = client . deploy ("0x60a06040523480156200001157600080fd5b506040516200472538038062004725833981016040819052620000349162000105565b806001600160a01b038116620000905760405162461bcd60e51b815260206004820152601a60248201527f53573a20574554485f206d7573742062652070726f7669646564000000000000604482015260640160405180910390fd5b6001600160a01b0316608052620000a733620000b3565b506001600f5562000137565b600e80546001600160a01b038381166001600160a01b0319831681179093556040519116919082907f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e090600090a35050565b6000602082840312156200011857600080fd5b81516001600160a01b03811681146200013057600080fd5b9392505050565b6080516145c462000161600039600081816103e10152818161214c01526125f701526145c46000f3fe6080604052600436106101445760003560e01c8063719e2454116100c0578063e048585911610074578063e21a25dc11610059578063e21a25dc14610403578063f2fde38b14610423578063ff0f99c61461044357600080fd5b8063e0485859146103af578063e0af3616146103cf57600080fd5b80638da5cb5b116100a55780638da5cb5b146102c3578063b376955f146102f5578063b51942511461031557600080fd5b8063719e2454146102905780638369bd84146102b057600080fd5b80633f15c2181161011757806341ab3d94116100fc57806341ab3d941461024857806370e847bd1461025b578063715018a61461027b57600080fd5b80633f15c218146102205780633f4106181461023557600080fd5b806301ffc9a714610149578063150b7a021461017e578063311ef7ae146101db578063379fcc57146101fd575b600080fd5b34801561015557600080fd5b50610169610164366004613ff2565b61048d565b60405190151581526020015b60405180910390f35b34801561018a57600080fd5b506101c26101993660046140a1565b7f150b7a0200000000000000000000000000000000000000000000000000000000949350505050565b6040516001600160e01b03199091168152602001610175565b3480156101e757600080fd5b506101fb6101f6366004614165565b610511565b005b34801561020957600080fd5b50610212610782565b604051908152602001610175565b34801561022c57600080fd5b50610212610793565b6101fb610243366004614191565b61079f565b6102126102563660046141d6565b610834565b34801561026757600080fd5b5061021261027636600461426b565b6108ad565b34801561028757600080fd5b506101fb6108ba565b34801561029c57600080fd5b506102126102ab36600461426b565b6108ce565b6102126102be366004614284565b6108db565b3480156102cf57600080fd5b50600e546001600160a01b03165b6040516001600160a01b039091168152602001610175565b34801561030157600080fd5b506101fb61031036600461426b565b610958565b34801561032157600080fd5b5061033561033036600461426b565b6109ca565b6040805185516001600160a01b0390811682526020808801519083015286830151811692820192909252606080870151908201526080808701516001600160801b039081169183019190915260a096870151169581019590955260c085019390935260e08401919091521661010082015261012001610175565b3480156103bb57600080fd5b506101fb6103ca36600461426b565b610a99565b3480156103db57600080fd5b506102dd7f000000000000000000000000000000000000000000000000000000000000000081565b34801561040f57600080fd5b506101fb61041e3660046142bf565b610b03565b34801561042f57600080fd5b506101fb61043e3660046142eb565b610df2565b34801561044f57600080fd5b5061046361045e36600461426b565b610e82565b604080516001600160a01b0390951685526020850193909352918301526060820152608001610175565b60007fcc59ac4a000000000000000000000000000000000000000000000000000000006001600160e01b0319831614806104f057507f52e839bf000000000000000000000000000000000000000000000000000000006001600160e01b03198316145b8061050b575063188f7bd760e11b6001600160e01b03198316145b92915050565b80600c5460001461058f5760405162461bcd60e51b815260206004820152602160248201527f54414b45523a2074616b657246726f6d2c207265656e7472616e63792063616c60448201527f6c0000000000000000000000000000000000000000000000000000000000000060648201526084015b60405180910390fd5b600c8190556040517fff0f99c6000000000000000000000000000000000000000000000000000000008152600481018390526000908190819081906001600160a01b0388169063ff0f99c69060240160806040518083038186803b1580156105f657600080fd5b505afa15801561060a573d6000803e3d6000fd5b505050506040513d601f19601f8201168201806040525081019061062e9190614318565b93509350935093506000806000806106468888610f58565b929650909450925090506001600160a01b038116156106e2576001600160a01b03811633146106dd5760405162461bcd60e51b815260206004820152602a60248201527f54414b45525f524543563a206f6e6c79206465785f2063616e2063616c6c207460448201527f686973206d6574686f64000000000000000000000000000000000000000000006064820152608401610586565b610760565b6001600160a01b038b1633146107605760405162461bcd60e51b815260206004820152603460248201527f54414b45525f524543563a206f6e6c792074616b657220636f6e74726163742060448201527f63616e2063616c6c2074686973206d6574686f640000000000000000000000006064820152608401610586565b6107708b8b898787878c8c611023565b50506000600c55505050505050505050565b600061078e6001611119565b905090565b600061078e6006611119565b6107a882611123565b6000828152600560205260409020546001600160a01b031680156107ed576107d0838261117a565b600083815260056020526040902080546001600160a01b03191690555b6001600160a01b0382161561082f576108078383346112da565b600083815260056020526040902080546001600160a01b0319166001600160a01b0384161790555b505050565b600061083e611523565b6002600f5414156108915760405162461bcd60e51b815260206004820152601f60248201527f5265656e7472616e637947756172643a207265656e7472616e742063616c6c006044820152606401610586565b6002600f556108a133848461157d565b6001600f559392505050565b600061050b6006836116d4565b6108c2611523565b6108cc60006116e7565b565b600061050b6001836116d4565b60006108e5611523565b6002600f5414156109385760405162461bcd60e51b815260206004820152601f60248201527f5265656e7472616e637947756172643a207265656e7472616e742063616c6c006044820152606401610586565b6002600f5561094a3386868686611739565b6001600f5595945050505050565b610960611523565b6002600f5414156109b35760405162461bcd60e51b815260206004820152601f60248201527f5265656e7472616e637947756172643a207265656e7472616e742063616c6c006044820152606401610586565b6002600f556109c23382611c0b565b506001600f55565b6040805160c081018252600080825260208201819052918101829052606081018290526080810182905260a08101829052908080610a0785611123565b505050600082815260208181526040808320815160c08101835281546001600160a01b039081168252600183015482860152600283015481168285015260038084015460608401526004938401546001600160801b038082166080860152600160801b9091041660a0840152978652968452828520549184528285205460059094529190932054909592949193501690565b610aa1611523565b6002600f541415610af45760405162461bcd60e51b815260206004820152601f60248201527f5265656e7472616e637947756172643a207265656e7472616e742063616c6c006044820152606401610586565b6002600f556109c23382611df7565b82610b0d81611fa9565b600b5415610b835760405162461bcd60e51b815260206004820152602560248201527f54414b45523a2074616b657243616c6c6261636b2c207265656e7472616e637960448201527f2063616c6c0000000000000000000000000000000000000000000000000000006064820152608401610586565b600b819055600084815260086020908152604080832060099092529091205460038201548491610bb29161436d565b1015610c265760405162461bcd60e51b815260206004820152603e60248201527f54414b45523a20726573706f6e736550726963655175616e746974794f72496460448201527f5f206f766572666c6f77206f722074616b65722066756c6c66696c6c656400006064820152608401610586565b80546001600160a01b03163314610c3c57600080fd5b8060020154841115610cb65760405162461bcd60e51b815260206004820152602860248201527f54414b45523a20726573706f6e7365536b755175616e746974794f7249645f2060448201527f6f766572666c6f770000000000000000000000000000000000000000000000006064820152608401610586565b8060030154831115610d305760405162461bcd60e51b815260206004820152602a60248201527f54414b45523a20726573706f6e736550726963655175616e746974794f72496460448201527f5f206f766572666c6f77000000000000000000000000000000000000000000006064820152608401610586565b805460048201805460088401549192610d66926001600160a01b03918216929091169088906001600160801b0316156000612000565b50815460028201546004830154610d9f926001600160a01b039081169216908790600160801b90046001600160801b0316156000612487565b6000868152600a602052604081208054879290610dbd908490614384565b909155505060008681526009602052604081208054869290610de0908490614384565b90915550506000600b55505050505050565b610dfa611523565b6001600160a01b038116610e765760405162461bcd60e51b815260206004820152602660248201527f4f776e61626c653a206e6577206f776e657220697320746865207a65726f206160448201527f64647265737300000000000000000000000000000000000000000000000000006064820152608401610586565b610e7f816116e7565b50565b600080600080610e9185611fa9565b50505060009182525060086020818152604092839020835160a0808201865282546001600160a01b03908116835260018401548386019081526002850154848901908152600386015460608087019182528a5160c081018c5260048901548616815260058901549981019990995260068801549094169988019990995260078601549287019290925293909501546001600160801b03808216608087810191909152600160801b909204169185019190915281019290925290519051915192519093919291565b6040805160c081018252600080825260208201819052918101829052606081018290526080810182905260a08101829052908080306001600160a01b0387161461100a5760405162461bcd60e51b815260206004820152603260248201527f53573a206f6e6c7920737570706f7274207175657279207468697320636f6e7460448201527f726163742773206d616b6572206f7264657200000000000000000000000000006064820152608401610586565b611013856109ca565b9299919850965090945092505050565b600080611033878787878761281d565b91509150600080611046898d8686612c07565b6040517fe21a25dc000000000000000000000000000000000000000000000000000000008152600481018e9052602481018790526044810186905291935091506001600160a01b038d169063e21a25dc90606401600060405180830381600087803b1580156110b457600080fd5b505af11580156110c8573d6000803e3d6000fd5b505050506000806110db8b8f8888612ffa565b909250905060006110ec838661436d565b905060006110fa858461436d565b90506111078e8383613355565b50505050505050505050505050505050565b600061050b825490565b61112e600182613360565b610e7f5760405162461bcd60e51b815260206004820152601e60248201527f4d414b45523a206d616b657220696420646f6573206e6f7420657869737400006044820152606401610586565b600082116111ca5760405162461bcd60e51b815260206004820152601860248201527f4d414b45523a206d616b6572206964206d757374203e203000000000000000006044820152606401610586565b6001600160a01b03811661122b5760405162461bcd60e51b815260206004820152602260248201527f4d414b45523a206c69737420746f206465782061646472657373206973207a65604482015261726f60f01b6064820152608401610586565b6040517f964bc33f000000000000000000000000000000000000000000000000000000008152600481018390526001600160a01b0382169063964bc33f90602401600060405180830381600087803b15801561128657600080fd5b505af115801561129a573d6000803e3d6000fd5b50506040516001600160a01b038416925084915030907fa86b2d999fb5a343b3ec7e35d7aa081d880ca9c59b28f1cb17105eadd90206d290600090a45050565b6000831161132a5760405162461bcd60e51b815260206004820152601860248201527f4d414b45523a206d616b6572206964206d757374203e203000000000000000006044820152606401610586565b6001600160a01b03821661138b5760405162461bcd60e51b815260206004820152602260248201527f4d414b45523a206c69737420746f206465782061646472657373206973207a65604482015261726f60f01b6064820152608401610586565b6040517f6fcca69b0000000000000000000000000000000000000000000000000000000081523060048201526000906001600160a01b03841690636fcca69b9060240160206040518083038186803b1580156113e657600080fd5b505afa1580156113fa573d6000803e3d6000fd5b505050506040513d601f19601f8201168201806040525081019061141e919061439c565b9050818111156114705760405162461bcd60e51b815260206004820152601f60248201527f4d414b45523a206c697374206465782066656520696e737566666963656e74006044820152606401610586565b6040517f80c9419e000000000000000000000000000000000000000000000000000000008152600481018590526001600160a01b038416906380c9419e9084906024016000604051808303818588803b1580156114cc57600080fd5b505af11580156114e0573d6000803e3d6000fd5b50506040516001600160a01b03871693508792503091507f6ca48fec369c94ac21d3ad9f5f459cda26bb2501fd6f72f5031cab0a20dac17490600090a450505050565b600e546001600160a01b031633146108cc5760405162461bcd60e51b815260206004820181905260248201527f4f776e61626c653a2063616c6c6572206973206e6f7420746865206f776e65726044820152606401610586565b600061158883613378565b60006115b0858560000151866020015187608001516001600160801b03166000146001612000565b6115ba903461436d565b90506115c4613702565b60008181526020818152604091829020875181546001600160a01b039182166001600160a01b031991821617835592890151600180840191909155938901516002830180549190921693169290921790915560608701516003820155608087015160a08801516001600160801b03908116600160801b029116176004909101559092506116519083613719565b506001600160a01b038316156116945761166c8284836112da565b600082815260056020526040902080546001600160a01b0319166001600160a01b0385161790555b6040516001600160a01b03841690839030907f5a50de70ddb20b525a4d329ceb02b1cbdd932b0d7badd3c5a375f4a377986b7790600090a4509392505050565b60006116e08383613725565b9392505050565b600e80546001600160a01b038381166001600160a01b0319831681179093556040519116919082907f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e090600090a35050565b60006001600160a01b0385166117915760405162461bcd60e51b815260206004820152601b60248201527f54414b45523a206d616b65725f206973206164647265737328302900000000006044820152606401610586565b600084116117e15760405162461bcd60e51b815260206004820152601360248201527f54414b45523a206d616b657249645f203e2030000000000000000000000000006044820152606401610586565b6000831161183c5760405162461bcd60e51b815260206004820152602260248201527f54414b45523a2072657175657374536b755175616e746974794f7249645f203e604482015261020360f41b6064820152608401610586565b600083116118975760405162461bcd60e51b815260206004820152602260248201527f54414b45523a2072657175657374536b755175616e746974794f7249645f203e604482015261020360f41b6064820152608401610586565b600080600080886001600160a01b031663b5194251896040518263ffffffff1660e01b81526004016118cb91815260200190565b6101206040518083038186803b1580156118e457600080fd5b505afa1580156118f8573d6000803e3d6000fd5b505050506040513d601f19601f8201168201806040525081019061191c91906143b5565b935093509350935061192d84613378565b60008061193d8686868c8c61281d565b915091506119638c8760400151838960a001516001600160801b03166000146001612000565b5061196c613702565b96506040518060a001604052808c6001600160a01b031681526020018b8152602001838152602001828152602001878152506008600089815260200190815260200160002060008201518160000160006101000a8154816001600160a01b0302191690836001600160a01b0316021790555060208201518160010155604082015181600201556060820151816003015560808201518160040160008201518160000160006101000a8154816001600160a01b0302191690836001600160a01b031602179055506020820151816001015560408201518160020160006101000a8154816001600160a01b0302191690836001600160a01b031602179055506060820151816003015560808201518160040160006101000a8154816001600160801b0302191690836001600160801b0316021790555060a08201518160040160106101000a8154816001600160801b0302191690836001600160801b031602179055505050905050611ae98760001b600661371990919063ffffffff16565b506040805183815260208101839052889130917f3303acdb363a7cefc28b6597f7c949bbd8157802821cde3978145652137dfeea910160405180910390a36001600160a01b03831615611b9b5760405163188f7bd760e11b8152306004820152602481018890526001600160a01b0384169063311ef7ae90604401600060405180830381600087803b158015611b7e57600080fd5b505af1158015611b92573d6000803e3d6000fd5b50505050611bfc565b60405163188f7bd760e11b8152306004820152602481018890526001600160a01b038c169063311ef7ae90604401600060405180830381600087803b158015611be357600080fd5b505af1158015611bf7573d6000803e3d6000fd5b505050505b50505050505095945050505050565b611c1481611fa9565b6000818152600860208181526040808420815160a0808201845282546001600160a01b03908116835260018401548387015260028401548386015260038401546060808501918252865160c0810188526004870154841681526005870154818a015260068701549093168388015260078601549083015293909601546001600160801b03808216608089810191909152600160801b90920416918701919091528101859052868652600a845282862054600990945291852054905191949091611cde90839061436d565b600087815260086020818152604080842080546001600160a01b0319908116825560018201869055600282018690556003820186905560048201805482169055600582018690556006808301805490921690915560078201869055930184905560098252808420849055600a909152822091909155909150611d60908761374f565b508215611d8957611d898785600001518587608001516001600160801b03166000146001612487565b8015611db157611db1878560400151838760a001516001600160801b03166000146001612487565b6040805184815260208101849052879130917f3cdf4e4024a4439a49d692a46a976c5e2da851bee35296f4e9cb79c24682f960910160405180910390a350505050505050565b611e0081611123565b600081815260208181526040808320815160c08101835281546001600160a01b039081168252600183015482860190815260028401549091168285015260038084015460608401526004909301546001600160801b038082166080850152600160801b9091041660a083015286865291909352908320549051919291611e86919061436d565b600084815260046020818152604080842080546005808552838720805488875285892080546001600160a01b0319908116825560018281018c90556002830180548316905560038084018d905592909a018b905590885295892089905597909355909352805490911690559293506001600160a01b0390911690611f0a908661374f565b508215611f3357611f338685600001518587608001516001600160801b03166000146001612487565b8115611f5b57611f5b868560400151848760a001516001600160801b03166000146001612487565b6001600160a01b03811615611f7457611f74858261117a565b604051859030907f4c90fb6cdcc07be6f74e51e9bbec088471af4713f67d182090dfdd9ba9d1eaa390600090a3505050505050565b611fb4600682613360565b610e7f5760405162461bcd60e51b815260206004820152601e60248201527f54414b45523a2074616b657220696420646f6573206e6f7420657869737400006044820152606401610586565b60006001600160a01b0386166120585760405162461bcd60e51b815260206004820181905260248201527f53573a20696e76616c696420706172616d205f6465706f7369742366726f6d5f6044820152606401610586565b6001600160a01b0385166120d45760405162461bcd60e51b815260206004820152602160248201527f53573a20696e76616c696420706172616d205f6465706f73697423617373657460448201527f5f000000000000000000000000000000000000000000000000000000000000006064820152608401610586565b6000841161214a5760405162461bcd60e51b815260206004820152602660248201527f53573a20696e76616c696420706172616d205f6465706f7369742376616c756560448201527f4f7249645f2000000000000000000000000000000000000000000000000000006064820152608401610586565b7f00000000000000000000000000000000000000000000000000000000000000006001600160a01b0316856001600160a01b031614156123f557826121d15760405162461bcd60e51b815260206004820152601960248201527f465a57543a205745544820697320657263323020746f6b656e000000000000006044820152606401610586565b81156123f5578334101561224d5760405162461bcd60e51b815260206004820152603460248201527f465a57543a206465706f736974206e617469766520746f6b656e20776974682060448201527f696e737566666963656e74207061796d656e742e0000000000000000000000006064820152608401610586565b6040516370a0823160e01b81523060048201526000906001600160a01b038716906370a082319060240160206040518083038186803b15801561228f57600080fd5b505afa1580156122a3573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906122c7919061439c565b9050856001600160a01b031663d0e30db0866040518263ffffffff1660e01b81526004016000604051808303818588803b15801561230457600080fd5b505af1158015612318573d6000803e3d6000fd5b50506040516370a0823160e01b81523060048201526001600160a01b038a1693506370a082319250602401905060206040518083038186803b15801561235d57600080fd5b505afa158015612371573d6000803e3d6000fd5b505050506040513d601f19601f82011682018060405250810190612395919061439c565b61239f8683614384565b146123ec5760405162461bcd60e51b815260206004820152601960248201527f465a57543a2057455448206465706f736974206661696c6564000000000000006044820152606401610586565b8491505061247e565b8215612415576124106001600160a01b03861687308761375b565b61247e565b604051632142170760e11b81526001600160a01b038781166004830152306024830152604482018690528616906342842e0e90606401600060405180830381600087803b15801561246557600080fd5b505af1158015612479573d6000803e3d6000fd5b505050505b95945050505050565b6001600160a01b0385166125035760405162461bcd60e51b815260206004820152602560248201527f53573a20696e76616c696420706172616d205f7769746864726177237265636960448201527f70656e745f0000000000000000000000000000000000000000000000000000006064820152608401610586565b6001600160a01b03841661257f5760405162461bcd60e51b815260206004820152602260248201527f53573a20696e76616c696420706172616d205f7769746864726177236173736560448201527f745f0000000000000000000000000000000000000000000000000000000000006064820152608401610586565b600083116125f55760405162461bcd60e51b815260206004820152602760248201527f53573a20696e76616c696420706172616d205f77697468647261772376616c7560448201527f654f7249645f20000000000000000000000000000000000000000000000000006064820152608401610586565b7f00000000000000000000000000000000000000000000000000000000000000006001600160a01b0316846001600160a01b0316141561278e578161267c5760405162461bcd60e51b815260206004820152601960248201527f465a57543a205745544820697320657263323020746f6b656e000000000000006044820152606401610586565b801561278e576040517f2e1a7d4d0000000000000000000000000000000000000000000000000000000081526004810184905247906001600160a01b03861690632e1a7d4d90602401600060405180830381600087803b1580156126df57600080fd5b505af11580156126f3573d6000803e3d6000fd5b505050504784826127049190614384565b146127515760405162461bcd60e51b815260206004820152601a60248201527f465a57543a2057455448207769746864726177206661696c65640000000000006044820152606401610586565b6040516001600160a01b0387169085156108fc029086906000818181858888f19350505050158015612787573d6000803e3d6000fd5b5050612816565b81156127ad576127a86001600160a01b03851686856137fa565b612816565b604051632142170760e11b81523060048201526001600160a01b038681166024830152604482018590528516906342842e0e90606401600060405180830381600087803b1580156127fd57600080fd5b505af1158015612811573d6000803e3d6000fd5b505050505b5050505050565b6000806000841161288a5760405162461bcd60e51b815260206004820152603160248201527f53573a20696e626c6f636b20737761702072657175657374536b755175616e7460448201527006974794f7249645f206d757374203e203607c1b6064820152608401610586565b600083116129005760405162461bcd60e51b815260206004820152603360248201527f53573a20696e626c6f636b20737761702072657175657374507269636551756160448201527f6e746974794f7249645f206d757374203e2030000000000000000000000000006064820152608401610586565b60808701516001600160801b0316158015612926575060a08701516001600160801b0316155b156129e55785876020015161293b919061436d565b915081841015612949578391505b866020015182886060015161295e9190614467565b6129689190614486565b9050828110156129e05760405162461bcd60e51b815260206004820152602960248201527f53573a20696e737566666963656e74207265717565737450726963655175616e60448201527f746974794f7249645f00000000000000000000000000000000000000000000006064820152608401610586565b612bfd565b8515612a675760405162461bcd60e51b815260206004820152604560248201527f53573a2074726164696e67207061697220696e636c7564696e6720657263373260448201527f3120746f6b656e20646f6573206e6f7420737570706f7274207061727469616c606482015264081919585b60da1b608482015260a401610586565b8415612aea5760405162461bcd60e51b815260206004820152604660248201527f53573a202074726164696e67207061697220696e636c7564696e67206572633760448201527f323120746f6b656e20646f6573206e6f7420737570706f7274207061727469616064820152651b081919585b60d21b608482015260a401610586565b505060208501516060860151838214612b795760405162461bcd60e51b815260206004820152604560248201527f53573a2074726164696e67207061697220696e636c7564696e6720657263373260448201527f3120746f6b656e20646f6573206e6f7420737570706f7274207061727469616c606482015264081919585b60da1b608482015260a401610586565b808314612bfd5760405162461bcd60e51b815260206004820152604660248201527f53573a202074726164696e67207061697220696e636c7564696e67206572633760448201527f323120746f6b656e20646f6573206e6f7420737570706f7274207061727469616064820152651b081919585b60d21b608482015260a401610586565b9550959350505050565b600080612c1386613378565b6001600160a01b038516612c695760405162461bcd60e51b815260206004820152601f60248201527f53573a20696e626c6f636b207377617020746f5f2061646472657373283029006044820152606401610586565b60008411612cd35760405162461bcd60e51b815260206004820152603160248201527f53573a20696e626c6f636b207377617020617070726f7665536b755175616e7460448201527006974794f7249645f206d757374203e203607c1b6064820152608401610586565b60008311612d495760405162461bcd60e51b815260206004820152603260248201527f53573a20696e626c6f636b20737761702065787065637450726963655175616e60448201527f746974794f7249645f206d757374203e203000000000000000000000000000006064820152608401610586565b60808601516001600160801b0316612df2578551612d71906001600160a01b03168686613843565b85516040516370a0823160e01b81523060048201526001600160a01b03909116906370a082319060240160206040518083038186803b158015612db357600080fd5b505afa158015612dc7573d6000803e3d6000fd5b505050506040513d601f19601f82011682018060405250810190612deb919061439c565b9150612e5b565b855160405163095ea7b360e01b81526001600160a01b038781166004830152602482018790529091169063095ea7b390604401600060405180830381600087803b158015612e3f57600080fd5b505af1158015612e53573d6000803e3d6000fd5b505050508391505b60a08601516001600160801b0316612ef25760408087015190516370a0823160e01b81523060048201526001600160a01b03909116906370a08231906024015b60206040518083038186803b158015612eb357600080fd5b505afa158015612ec7573d6000803e3d6000fd5b505050506040513d601f19601f82011682018060405250810190612eeb919061439c565b9050612ff1565b60408681015190516331a9108f60e11b81526004810185905230916001600160a01b031690636352211e9060240160206040518083038186803b158015612f3857600080fd5b505afa158015612f4c573d6000803e3d6000fd5b505050506040513d601f19601f82011682018060405250810190612f7091906144a8565b6001600160a01b03161415612fed5760405162461bcd60e51b815260206004820152602a60248201527f53573a2065787065637450726963655175616e746974794f7249645f206f776e60448201527f65722069732073656c66000000000000000000000000000000000000000000006064820152608401610586565b5060005b94509492505050565b60008061300686613378565b6001600160a01b03851661305c5760405162461bcd60e51b815260206004820152601f60248201527f53573a20696e626c6f636b207377617020746f5f2061646472657373283029006044820152606401610586565b600084116130c65760405162461bcd60e51b815260206004820152603160248201527f53573a20696e626c6f636b207377617020617070726f7665536b755175616e7460448201527006974794f7249645f206d757374203e203607c1b6064820152608401610586565b6000831161313c5760405162461bcd60e51b815260206004820152603260248201527f53573a20696e626c6f636b20737761702065787065637450726963655175616e60448201527f746974794f7249645f206d757374203e203000000000000000000000000000006064820152608401610586565b60808601516001600160801b03166131e6578551613165906001600160a01b0316866000613843565b85516040516370a0823160e01b81523060048201526001600160a01b03909116906370a082319060240160206040518083038186803b1580156131a757600080fd5b505afa1580156131bb573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906131df919061439c565b915061327d565b85516040516331a9108f60e11b81526004810186905230916001600160a01b031690636352211e9060240160206040518083038186803b15801561322957600080fd5b505afa15801561323d573d6000803e3d6000fd5b505050506040513d601f19601f8201168201806040525081019061326191906144a8565b6001600160a01b031614156132785783915061327d565b600091505b60a08601516001600160801b03166132c15760408087015190516370a0823160e01b81523060048201526001600160a01b03909116906370a0823190602401612e9b565b60408681015190516331a9108f60e11b81526004810185905230916001600160a01b031690636352211e9060240160206040518083038186803b15801561330757600080fd5b505afa15801561331b573d6000803e3d6000fd5b505050506040513d601f19601f8201168201806040525081019061333f91906144a8565b6001600160a01b03161415612fed575081612ff1565b61082f838383613987565b600081815260018301602052604081205415156116e0565b80516001600160a01b03166133cf5760405162461bcd60e51b815260206004820152601760248201527f53573a20534b552061646472657373206973207a65726f0000000000000000006044820152606401610586565b60008160200151116134235760405162461bcd60e51b815260206004820152601760248201527f53573a20736b755175616e746974794f724964203e20300000000000000000006044820152606401610586565b60408101516001600160a01b03166134a25760405162461bcd60e51b8152602060048201526024808201527f53573a207061796d656e742063757272656e637920616464726573732069732060448201527f7a65726f000000000000000000000000000000000000000000000000000000006064820152608401610586565b60008160600151116134f65760405162461bcd60e51b815260206004820152601960248201527f53573a2070726963655175616e746974794f724964203e2030000000000000006044820152606401610586565b60808101516001600160801b0316156135fc5780516040516301ffc9a760e01b81526380ac58cd60e01b60048201526001600160a01b03909116906301ffc9a79060240160206040518083038186803b15801561355257600080fd5b505afa158015613566573d6000803e3d6000fd5b505050506040513d601f19601f8201168201806040525081019061358a91906144c5565b6135fc5760405162461bcd60e51b815260206004820152602760248201527f53573a20636865636b206d616b657220736b752074797065286572633732312960448201527f206661696c6564000000000000000000000000000000000000000000000000006064820152608401610586565b60a08101516001600160801b031615610e7f5780516040516301ffc9a760e01b81526380ac58cd60e01b60048201526001600160a01b03909116906301ffc9a79060240160206040518083038186803b15801561365857600080fd5b505afa15801561366c573d6000803e3d6000fd5b505050506040513d601f19601f8201168201806040525081019061369091906144c5565b610e7f5760405162461bcd60e51b815260206004820152603460248201527f53573a20636865636b206d616b6572207061796d656e742063757272656e637960448201527f20747970652865726337323129206661696c65640000000000000000000000006064820152608401610586565b6000613712600d80546001019055565b50600d5490565b60006116e08383613c33565b600082600001828154811061373c5761373c6144e7565b9060005260206000200154905092915050565b60006116e08383613c82565b6040516001600160a01b03808516602483015283166044820152606481018290526137f49085907f23b872dd00000000000000000000000000000000000000000000000000000000906084015b60408051601f198184030181529190526020810180517bffffffffffffffffffffffffffffffffffffffffffffffffffffffff166001600160e01b031990931692909217909152613d75565b50505050565b6040516001600160a01b03831660248201526044810182905261082f9084907fa9059cbb00000000000000000000000000000000000000000000000000000000906064016137a8565b8015806138e557506040517fdd62ed3e0000000000000000000000000000000000000000000000000000000081523060048201526001600160a01b03838116602483015284169063dd62ed3e9060440160206040518083038186803b1580156138ab57600080fd5b505afa1580156138bf573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906138e3919061439c565b155b6139575760405162461bcd60e51b815260206004820152603660248201527f5361666545524332303a20617070726f76652066726f6d206e6f6e2d7a65726f60448201527f20746f206e6f6e2d7a65726f20616c6c6f77616e6365000000000000000000006064820152608401610586565b6040516001600160a01b03831660248201526044810182905261082f90849063095ea7b360e01b906064016137a8565b61399083611123565b60008211613a065760405162461bcd60e51b815260206004820152602160248201527f53573a2073656e74536b755175616e746974794f7249645f206d757374203e2060448201527f30000000000000000000000000000000000000000000000000000000000000006064820152608401610586565b60008111613a7c5760405162461bcd60e51b815260206004820152602960248201527f53573a2072656365697665645061796d656e745175616e746974794f7249645f60448201527f206d757374203e203000000000000000000000000000000000000000000000006064820152608401610586565b600083815260208190526040902060048101546001600160801b031615613b2e57600084815260036020526040902054158015613abc5750828160010154145b613b2e5760405162461bcd60e51b815260206004820152603860248201527f4d414b45523a20736b752069732065726337323120746f6b656e20646f65732060448201527f6e6f7420737570706f7274207061727469616c206465616c00000000000000006064820152608401610586565b6004810154600160801b90046001600160801b031615613be757600084815260046020526040902054158015613b675750818160030154145b613be75760405162461bcd60e51b815260206004820152604560248201527f4d414b45523a207061796d656e742063757272656e637920697320657263373260448201527f3120746f6b656e20646f6573206e6f7420737570706f7274207061727469616c606482015264081919585b60da1b608482015260a401610586565b60008481526003602052604081208054859290613c05908490614384565b909155505060008481526004602052604081208054849290613c28908490614384565b909155505050505050565b6000818152600183016020526040812054613c7a5750815460018181018455600084815260208082209093018490558454848252828601909352604090209190915561050b565b50600061050b565b60008181526001830160205260408120548015613d6b576000613ca660018361436d565b8554909150600090613cba9060019061436d565b9050818114613d1f576000866000018281548110613cda57613cda6144e7565b9060005260206000200154905080876000018481548110613cfd57613cfd6144e7565b6000918252602080832090910192909255918252600188019052604090208390555b8554869080613d3057613d306144fd565b60019003818190600052602060002001600090559055856001016000868152602001908152602001600020600090556001935050505061050b565b600091505061050b565b6000613dca826040518060400160405280602081526020017f5361666545524332303a206c6f772d6c6576656c2063616c6c206661696c6564815250856001600160a01b0316613e5a9092919063ffffffff16565b80519091501561082f5780806020019051810190613de891906144c5565b61082f5760405162461bcd60e51b815260206004820152602a60248201527f5361666545524332303a204552433230206f7065726174696f6e20646964206e60448201527f6f742073756363656564000000000000000000000000000000000000000000006064820152608401610586565b6060613e698484600085613e71565b949350505050565b606082471015613ee95760405162461bcd60e51b815260206004820152602660248201527f416464726573733a20696e73756666696369656e742062616c616e636520666f60448201527f722063616c6c00000000000000000000000000000000000000000000000000006064820152608401610586565b6001600160a01b0385163b613f405760405162461bcd60e51b815260206004820152601d60248201527f416464726573733a2063616c6c20746f206e6f6e2d636f6e74726163740000006044820152606401610586565b600080866001600160a01b03168587604051613f5c919061453f565b60006040518083038185875af1925050503d8060008114613f99576040519150601f19603f3d011682016040523d82523d6000602084013e613f9e565b606091505b5091509150613fae828286613fb9565b979650505050505050565b60608315613fc85750816116e0565b825115613fd85782518084602001fd5b8160405162461bcd60e51b8152600401610586919061455b565b60006020828403121561400457600080fd5b81356001600160e01b0319811681146116e057600080fd5b6001600160a01b0381168114610e7f57600080fd5b634e487b7160e01b600052604160045260246000fd5b60405160c0810167ffffffffffffffff8111828210171561406a5761406a614031565b60405290565b604051601f8201601f1916810167ffffffffffffffff8111828210171561409957614099614031565b604052919050565b600080600080608085870312156140b757600080fd5b84356140c28161401c565b93506020858101356140d38161401c565b935060408601359250606086013567ffffffffffffffff808211156140f757600080fd5b818801915088601f83011261410b57600080fd5b81358181111561411d5761411d614031565b61412f601f8201601f19168501614070565b9150808252898482850101111561414557600080fd5b808484018584013760008482840101525080935050505092959194509250565b6000806040838503121561417857600080fd5b82356141838161401c565b946020939093013593505050565b600080604083850312156141a457600080fd5b8235915060208301356141b68161401c565b809150509250929050565b6001600160801b0381168114610e7f57600080fd5b60008082840360e08112156141ea57600080fd5b60c08112156141f857600080fd5b50614201614047565b833561420c8161401c565b81526020848101359082015260408401356142268161401c565b6040820152606084810135908201526080840135614243816141c1565b608082015260a0840135614256816141c1565b60a0820152915060c08301356141b68161401c565b60006020828403121561427d57600080fd5b5035919050565b6000806000806080858703121561429a57600080fd5b84356142a58161401c565b966020860135965060408601359560600135945092505050565b6000806000606084860312156142d457600080fd5b505081359360208301359350604090920135919050565b6000602082840312156142fd57600080fd5b81356116e08161401c565b80516143138161401c565b919050565b6000806000806080858703121561432e57600080fd5b84516143398161401c565b60208601516040870151606090970151919890975090945092505050565b634e487b7160e01b600052601160045260246000fd5b60008282101561437f5761437f614357565b500390565b6000821982111561439757614397614357565b500190565b6000602082840312156143ae57600080fd5b5051919050565b6000806000808486036101208112156143cd57600080fd5b60c08112156143db57600080fd5b506143e4614047565b85516143ef8161401c565b81526020868101519082015260408601516144098161401c565b6040820152606086810151908201526080860151614426816141c1565b608082015260a0860151614439816141c1565b60a082015260c086015160e08701519195509350915061445c6101008601614308565b905092959194509250565b600081600019048311821515161561448157614481614357565b500290565b6000826144a357634e487b7160e01b600052601260045260246000fd5b500490565b6000602082840312156144ba57600080fd5b81516116e08161401c565b6000602082840312156144d757600080fd5b815180151581146116e057600080fd5b634e487b7160e01b600052603260045260246000fd5b634e487b7160e01b600052603160045260246000fd5b60005b8381101561452e578181015183820152602001614516565b838111156137f45750506000910152565b60008251614551818460208701614513565b9190910192915050565b602081526000825180602084015261457a816040850160208701614513565b601f01601f1916919091016040019291505056fea26469706673582212204b6c76b6cc219eb9f65e911611afcec1715a7082eb8933ae4760f917f0e362b864736f6c63430008090033" , "Constructor(address)" , params , transfer_ops) . await . map_err (| err | std :: io :: Error :: new (std :: io :: ErrorKind :: Other , format ! ("{:#?}" , err))) ? ;
        Ok(Self::new(client, address))
    }
    pub async fn weth(&self) -> std::io::Result<(reweb3::runtimes::Address,)>
    where
        C: reweb3::runtimes::Client + Sync + Unpin,
    {
        let params = reweb3::runtimes::to_abi(()).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let call_result = self
            .client
            .call("_WETH()", &self.address, params)
            .await
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err)))?;
        Ok(reweb3::runtimes::from_abi(call_result).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?)
    }
    pub async fn approve_dex<MakerId, To, Ops>(
        &self,
        maker_id: MakerId,
        to: To,
        transfer_ops: Ops,
    ) -> std::io::Result<reweb3::runtimes::H256>
    where
        C: reweb3::runtimes::SignerWithProvider + Sync + Unpin,
        Ops: TryInto<reweb3::runtimes::TransferOptions>,
        Ops::Error: std::fmt::Debug + Send + 'static,
        MakerId: TryInto<reweb3::runtimes::U256>,
        MakerId::Error: std::fmt::Debug + Send + 'static,
        To: TryInto<reweb3::runtimes::Address>,
        To::Error: std::fmt::Debug + Send + 'static,
    {
        let transfer_ops = transfer_ops.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let maker_id = maker_id.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let to = to.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let params = reweb3::runtimes::to_abi((maker_id, to)).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        Ok(self
            .client
            .sign_and_send_transaction(
                "approveDex(uint256,address)",
                &self.address,
                params,
                transfer_ops,
            )
            .await
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err)))?)
    }
    pub async fn close_maker<MakerId, Ops>(
        &self,
        maker_id: MakerId,
        transfer_ops: Ops,
    ) -> std::io::Result<reweb3::runtimes::H256>
    where
        C: reweb3::runtimes::SignerWithProvider + Sync + Unpin,
        Ops: TryInto<reweb3::runtimes::TransferOptions>,
        Ops::Error: std::fmt::Debug + Send + 'static,
        MakerId: TryInto<reweb3::runtimes::U256>,
        MakerId::Error: std::fmt::Debug + Send + 'static,
    {
        let transfer_ops = transfer_ops.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let maker_id = maker_id.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let params = reweb3::runtimes::to_abi((maker_id,)).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        Ok(self
            .client
            .sign_and_send_transaction("closeMaker(uint256)", &self.address, params, transfer_ops)
            .await
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err)))?)
    }
    pub async fn close_taker<TakerId, Ops>(
        &self,
        taker_id: TakerId,
        transfer_ops: Ops,
    ) -> std::io::Result<reweb3::runtimes::H256>
    where
        C: reweb3::runtimes::SignerWithProvider + Sync + Unpin,
        Ops: TryInto<reweb3::runtimes::TransferOptions>,
        Ops::Error: std::fmt::Debug + Send + 'static,
        TakerId: TryInto<reweb3::runtimes::U256>,
        TakerId::Error: std::fmt::Debug + Send + 'static,
    {
        let transfer_ops = transfer_ops.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let taker_id = taker_id.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let params = reweb3::runtimes::to_abi((taker_id,)).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        Ok(self
            .client
            .sign_and_send_transaction("closeTaker(uint256)", &self.address, params, transfer_ops)
            .await
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err)))?)
    }
    pub async fn maker_by_index<Index>(
        &self,
        index: Index,
    ) -> std::io::Result<(reweb3::runtimes::U256,)>
    where
        C: reweb3::runtimes::Client + Sync + Unpin,
        Index: TryInto<reweb3::runtimes::U256>,
        Index::Error: std::fmt::Debug + Send + 'static,
    {
        let index = index.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let params = reweb3::runtimes::to_abi((index,)).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let call_result = self
            .client
            .call("makerByIndex(uint256)", &self.address, params)
            .await
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err)))?;
        Ok(reweb3::runtimes::from_abi(call_result).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?)
    }
    pub async fn maker_metadata<MakerId>(
        &self,
        maker_id: MakerId,
    ) -> std::io::Result<(
        tuples::IMakerMetadata,
        reweb3::runtimes::U256,
        reweb3::runtimes::U256,
        reweb3::runtimes::Address,
    )>
    where
        C: reweb3::runtimes::Client + Sync + Unpin,
        MakerId: TryInto<reweb3::runtimes::U256>,
        MakerId::Error: std::fmt::Debug + Send + 'static,
    {
        let maker_id = maker_id.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let params = reweb3::runtimes::to_abi((maker_id,)).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let call_result = self
            .client
            .call("makerMetadata(uint256)", &self.address, params)
            .await
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err)))?;
        Ok(reweb3::runtimes::from_abi(call_result).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?)
    }
    pub async fn mint_maker<Maker, Dex, Ops>(
        &self,
        maker: Maker,
        dex: Dex,
        transfer_ops: Ops,
    ) -> std::io::Result<reweb3::runtimes::H256>
    where
        C: reweb3::runtimes::SignerWithProvider + Sync + Unpin,
        Ops: TryInto<reweb3::runtimes::TransferOptions>,
        Ops::Error: std::fmt::Debug + Send + 'static,
        Maker: TryInto<tuples::IMakerMetadata>,
        Maker::Error: std::fmt::Debug + Send + 'static,
        Dex: TryInto<reweb3::runtimes::Address>,
        Dex::Error: std::fmt::Debug + Send + 'static,
    {
        let transfer_ops = transfer_ops.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let maker = maker.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let dex = dex.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let params = reweb3::runtimes::to_abi((maker, dex)).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        Ok(self
            .client
            .sign_and_send_transaction(
                "mintMaker((address,uint256,address,uint256,uint128,uint128),address)",
                &self.address,
                params,
                transfer_ops,
            )
            .await
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err)))?)
    }
    pub async fn mint_taker<Maker, MakerId, RequestSkuQuantityOrId, RequestPriceQuantityOrId, Ops>(
        &self,
        maker: Maker,
        maker_id: MakerId,
        request_sku_quantity_or_id: RequestSkuQuantityOrId,
        request_price_quantity_or_id: RequestPriceQuantityOrId,
        transfer_ops: Ops,
    ) -> std::io::Result<reweb3::runtimes::H256>
    where
        C: reweb3::runtimes::SignerWithProvider + Sync + Unpin,
        Ops: TryInto<reweb3::runtimes::TransferOptions>,
        Ops::Error: std::fmt::Debug + Send + 'static,
        Maker: TryInto<reweb3::runtimes::Address>,
        Maker::Error: std::fmt::Debug + Send + 'static,
        MakerId: TryInto<reweb3::runtimes::U256>,
        MakerId::Error: std::fmt::Debug + Send + 'static,
        RequestSkuQuantityOrId: TryInto<reweb3::runtimes::U256>,
        RequestSkuQuantityOrId::Error: std::fmt::Debug + Send + 'static,
        RequestPriceQuantityOrId: TryInto<reweb3::runtimes::U256>,
        RequestPriceQuantityOrId::Error: std::fmt::Debug + Send + 'static,
    {
        let transfer_ops = transfer_ops.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let maker = maker.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let maker_id = maker_id.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let request_sku_quantity_or_id = request_sku_quantity_or_id.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let request_price_quantity_or_id =
            request_price_quantity_or_id.try_into().map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
            })?;
        let params = reweb3::runtimes::to_abi((
            maker,
            maker_id,
            request_sku_quantity_or_id,
            request_price_quantity_or_id,
        ))
        .map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        Ok(self
            .client
            .sign_and_send_transaction(
                "mintTaker(address,uint256,uint256,uint256)",
                &self.address,
                params,
                transfer_ops,
            )
            .await
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err)))?)
    }
    pub async fn on_erc721_received<P0, P1, P2, P3, Ops>(
        &self,
        p0: P0,
        p1: P1,
        p2: P2,
        p3: P3,
        transfer_ops: Ops,
    ) -> std::io::Result<reweb3::runtimes::H256>
    where
        C: reweb3::runtimes::SignerWithProvider + Sync + Unpin,
        Ops: TryInto<reweb3::runtimes::TransferOptions>,
        Ops::Error: std::fmt::Debug + Send + 'static,
        P0: TryInto<reweb3::runtimes::Address>,
        P0::Error: std::fmt::Debug + Send + 'static,
        P1: TryInto<reweb3::runtimes::Address>,
        P1::Error: std::fmt::Debug + Send + 'static,
        P2: TryInto<reweb3::runtimes::U256>,
        P2::Error: std::fmt::Debug + Send + 'static,
        P3: TryInto<reweb3::runtimes::Bytes>,
        P3::Error: std::fmt::Debug + Send + 'static,
    {
        let transfer_ops = transfer_ops.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let p0 = p0.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let p1 = p1.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let p2 = p2.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let p3 = p3.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let params = reweb3::runtimes::to_abi((p0, p1, p2, p3)).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        Ok(self
            .client
            .sign_and_send_transaction(
                "onERC721Received(address,address,uint256,bytes)",
                &self.address,
                params,
                transfer_ops,
            )
            .await
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err)))?)
    }
    pub async fn owner(&self) -> std::io::Result<(reweb3::runtimes::Address,)>
    where
        C: reweb3::runtimes::Client + Sync + Unpin,
    {
        let params = reweb3::runtimes::to_abi(()).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let call_result = self
            .client
            .call("owner()", &self.address, params)
            .await
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err)))?;
        Ok(reweb3::runtimes::from_abi(call_result).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?)
    }
    pub async fn renounce_ownership<Ops>(
        &self,
        transfer_ops: Ops,
    ) -> std::io::Result<reweb3::runtimes::H256>
    where
        C: reweb3::runtimes::SignerWithProvider + Sync + Unpin,
        Ops: TryInto<reweb3::runtimes::TransferOptions>,
        Ops::Error: std::fmt::Debug + Send + 'static,
    {
        let transfer_ops = transfer_ops.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let params = reweb3::runtimes::to_abi(()).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        Ok(self
            .client
            .sign_and_send_transaction("renounceOwnership()", &self.address, params, transfer_ops)
            .await
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err)))?)
    }
    pub async fn supports_interface<InterfaceId>(
        &self,
        interface_id: InterfaceId,
    ) -> std::io::Result<(bool,)>
    where
        C: reweb3::runtimes::Client + Sync + Unpin,
        InterfaceId: TryInto<reweb3::runtimes::Bytes4>,
        InterfaceId::Error: std::fmt::Debug + Send + 'static,
    {
        let interface_id = interface_id.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let params = reweb3::runtimes::to_abi((interface_id,)).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let call_result = self
            .client
            .call("supportsInterface(bytes4)", &self.address, params)
            .await
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err)))?;
        Ok(reweb3::runtimes::from_abi(call_result).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?)
    }
    pub async fn taker_by_index<Index>(
        &self,
        index: Index,
    ) -> std::io::Result<(reweb3::runtimes::U256,)>
    where
        C: reweb3::runtimes::Client + Sync + Unpin,
        Index: TryInto<reweb3::runtimes::U256>,
        Index::Error: std::fmt::Debug + Send + 'static,
    {
        let index = index.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let params = reweb3::runtimes::to_abi((index,)).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let call_result = self
            .client
            .call("takerByIndex(uint256)", &self.address, params)
            .await
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err)))?;
        Ok(reweb3::runtimes::from_abi(call_result).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?)
    }
    pub async fn taker_callback<TakerId, ResponseSkuQuantityOrId, ResponsePriceQuantityOrId, Ops>(
        &self,
        taker_id: TakerId,
        response_sku_quantity_or_id: ResponseSkuQuantityOrId,
        response_price_quantity_or_id: ResponsePriceQuantityOrId,
        transfer_ops: Ops,
    ) -> std::io::Result<reweb3::runtimes::H256>
    where
        C: reweb3::runtimes::SignerWithProvider + Sync + Unpin,
        Ops: TryInto<reweb3::runtimes::TransferOptions>,
        Ops::Error: std::fmt::Debug + Send + 'static,
        TakerId: TryInto<reweb3::runtimes::U256>,
        TakerId::Error: std::fmt::Debug + Send + 'static,
        ResponseSkuQuantityOrId: TryInto<reweb3::runtimes::U256>,
        ResponseSkuQuantityOrId::Error: std::fmt::Debug + Send + 'static,
        ResponsePriceQuantityOrId: TryInto<reweb3::runtimes::U256>,
        ResponsePriceQuantityOrId::Error: std::fmt::Debug + Send + 'static,
    {
        let transfer_ops = transfer_ops.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let taker_id = taker_id.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let response_sku_quantity_or_id =
            response_sku_quantity_or_id.try_into().map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
            })?;
        let response_price_quantity_or_id =
            response_price_quantity_or_id.try_into().map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
            })?;
        let params = reweb3::runtimes::to_abi((
            taker_id,
            response_sku_quantity_or_id,
            response_price_quantity_or_id,
        ))
        .map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        Ok(self
            .client
            .sign_and_send_transaction(
                "takerCallback(uint256,uint256,uint256)",
                &self.address,
                params,
                transfer_ops,
            )
            .await
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err)))?)
    }
    pub async fn taker_from<From, TakerId, Ops>(
        &self,
        from: From,
        taker_id: TakerId,
        transfer_ops: Ops,
    ) -> std::io::Result<reweb3::runtimes::H256>
    where
        C: reweb3::runtimes::SignerWithProvider + Sync + Unpin,
        Ops: TryInto<reweb3::runtimes::TransferOptions>,
        Ops::Error: std::fmt::Debug + Send + 'static,
        From: TryInto<reweb3::runtimes::Address>,
        From::Error: std::fmt::Debug + Send + 'static,
        TakerId: TryInto<reweb3::runtimes::U256>,
        TakerId::Error: std::fmt::Debug + Send + 'static,
    {
        let transfer_ops = transfer_ops.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let from = from.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let taker_id = taker_id.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let params = reweb3::runtimes::to_abi((from, taker_id)).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        Ok(self
            .client
            .sign_and_send_transaction(
                "takerFrom(address,uint256)",
                &self.address,
                params,
                transfer_ops,
            )
            .await
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err)))?)
    }
    pub async fn taker_metadata<TakerId>(
        &self,
        taker_id: TakerId,
    ) -> std::io::Result<(
        reweb3::runtimes::Address,
        reweb3::runtimes::U256,
        reweb3::runtimes::U256,
        reweb3::runtimes::U256,
    )>
    where
        C: reweb3::runtimes::Client + Sync + Unpin,
        TakerId: TryInto<reweb3::runtimes::U256>,
        TakerId::Error: std::fmt::Debug + Send + 'static,
    {
        let taker_id = taker_id.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let params = reweb3::runtimes::to_abi((taker_id,)).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let call_result = self
            .client
            .call("takerMetadata(uint256)", &self.address, params)
            .await
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err)))?;
        Ok(reweb3::runtimes::from_abi(call_result).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?)
    }
    pub async fn total_supply_of_maker(&self) -> std::io::Result<(reweb3::runtimes::U256,)>
    where
        C: reweb3::runtimes::Client + Sync + Unpin,
    {
        let params = reweb3::runtimes::to_abi(()).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let call_result = self
            .client
            .call("totalSupplyOfMaker()", &self.address, params)
            .await
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err)))?;
        Ok(reweb3::runtimes::from_abi(call_result).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?)
    }
    pub async fn total_supply_of_taker(&self) -> std::io::Result<(reweb3::runtimes::U256,)>
    where
        C: reweb3::runtimes::Client + Sync + Unpin,
    {
        let params = reweb3::runtimes::to_abi(()).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let call_result = self
            .client
            .call("totalSupplyOfTaker()", &self.address, params)
            .await
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err)))?;
        Ok(reweb3::runtimes::from_abi(call_result).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?)
    }
    pub async fn transfer_ownership<NewOwner, Ops>(
        &self,
        new_owner: NewOwner,
        transfer_ops: Ops,
    ) -> std::io::Result<reweb3::runtimes::H256>
    where
        C: reweb3::runtimes::SignerWithProvider + Sync + Unpin,
        Ops: TryInto<reweb3::runtimes::TransferOptions>,
        Ops::Error: std::fmt::Debug + Send + 'static,
        NewOwner: TryInto<reweb3::runtimes::Address>,
        NewOwner::Error: std::fmt::Debug + Send + 'static,
    {
        let transfer_ops = transfer_ops.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let new_owner = new_owner.try_into().map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        let params = reweb3::runtimes::to_abi((new_owner,)).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}", err))
        })?;
        Ok(self
            .client
            .sign_and_send_transaction(
                "transferOwnership(address)",
                &self.address,
                params,
                transfer_ops,
            )
            .await
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}", err)))?)
    }
}
