use std::net::Ipv4Addr;

use crate::{
    error::{DecodeError, EncodeError},
    frame::MessageFrame,
    primitives::{Reader, Writer},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    ClientToServer,
    ServerToClient,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ServerCode {
    Login = 1,
    SetWaitPort = 2,
    GetPeerAddress = 3,
    WatchUser = 5,
    UnwatchUser = 6,
    GetUserStatus = 7,
    IgnoreUser = 11,
    UnignoreUser = 12,
    SayChatroom = 13,
    JoinRoom = 14,
    LeaveRoom = 15,
    UserJoinedRoom = 16,
    UserLeftRoom = 17,
    ConnectToPeer = 18,
    MessageUser = 22,
    MessageAcked = 23,
    FileSearchRoom = 25,
    FileSearch = 26,
    SetStatus = 28,
    ServerPing = 32,
    SendConnectToken = 33,
    SendDownloadSpeed = 34,
    SharedFoldersFiles = 35,
    GetUserStats = 36,
    UploadSlotsFull = 40,
    Relogged = 41,
    UserSearch = 42,
    SimilarRecommendations = 50,
    AddThingILike = 51,
    RemoveThingILike = 52,
    Recommendations = 54,
    MyRecommendations = 55,
    GlobalRecommendations = 56,
    UserInterests = 57,
    AdminCommand = 58,
    PlaceInLineRequest = 59,
    PlaceInLineResponse = 60,
    RoomAdded = 62,
    RoomRemoved = 63,
    RoomList = 64,
    ExactFileSearch = 65,
    AdminMessage = 66,
    GlobalUserList = 67,
    TunneledMessage = 68,
    PrivilegedUsers = 69,
    HaveNoParent = 71,
    ParentIp = 73,
    ParentMinSpeed = 83,
    ParentSpeedRatio = 84,
    ParentInactivityTimeout = 86,
    SearchInactivityTimeout = 87,
    MinParentsInCache = 88,
    DistribPingInterval = 90,
    AddToPrivileged = 91,
    CheckPrivileges = 92,
    EmbeddedMessage = 93,
    AcceptChildren = 100,
    PossibleParents = 102,
    WishlistSearch = 103,
    WishlistInterval = 104,
    SimilarUsers = 110,
    ItemRecommendations = 111,
    ItemSimilarUsers = 112,
    RoomTickers = 113,
    RoomTickerAdded = 114,
    RoomTickerRemoved = 115,
    SetRoomTicker = 116,
    AddThingIHate = 117,
    RemoveThingIHate = 118,
    RoomSearch = 120,
    SendUploadSpeed = 121,
    UserPrivileged = 122,
    GivePrivileges = 123,
    NotifyPrivileges = 124,
    AckNotifyPrivileges = 125,
    BranchLevel = 126,
    BranchRoot = 127,
    ChildDepth = 129,
    ResetDistributed = 130,
    RoomMembers = 133,
    AddRoomMember = 134,
    RemoveRoomMember = 135,
    CancelRoomMembership = 136,
    CancelRoomOwnership = 137,
    RoomSomething = 138,
    RoomMembershipGranted = 139,
    RoomMembershipRevoked = 140,
    EnableRoomInvitations = 141,
    ChangePassword = 142,
    AddRoomOperator = 143,
    RemoveRoomOperator = 144,
    RoomOperatorshipGranted = 145,
    RoomOperatorshipRevoked = 146,
    RoomOperators = 148,
    MessageUsers = 149,
    JoinGlobalRoom = 150,
    LeaveGlobalRoom = 151,
    GlobalRoomMessage = 152,
    RelatedSearch = 153,
    ExcludedSearchPhrases = 160,
    CantConnectToPeer = 1001,
    CantCreateRoom = 1002,
    CantJoinRoom = 1003,
}

impl ServerCode {
    pub const ALL: &'static [Self] = &[
        Self::Login,
        Self::SetWaitPort,
        Self::GetPeerAddress,
        Self::WatchUser,
        Self::UnwatchUser,
        Self::GetUserStatus,
        Self::IgnoreUser,
        Self::UnignoreUser,
        Self::SayChatroom,
        Self::JoinRoom,
        Self::LeaveRoom,
        Self::UserJoinedRoom,
        Self::UserLeftRoom,
        Self::ConnectToPeer,
        Self::MessageUser,
        Self::MessageAcked,
        Self::FileSearchRoom,
        Self::FileSearch,
        Self::SetStatus,
        Self::ServerPing,
        Self::SendConnectToken,
        Self::SendDownloadSpeed,
        Self::SharedFoldersFiles,
        Self::GetUserStats,
        Self::UploadSlotsFull,
        Self::Relogged,
        Self::UserSearch,
        Self::SimilarRecommendations,
        Self::AddThingILike,
        Self::RemoveThingILike,
        Self::Recommendations,
        Self::MyRecommendations,
        Self::GlobalRecommendations,
        Self::UserInterests,
        Self::AdminCommand,
        Self::PlaceInLineRequest,
        Self::PlaceInLineResponse,
        Self::RoomAdded,
        Self::RoomRemoved,
        Self::RoomList,
        Self::ExactFileSearch,
        Self::AdminMessage,
        Self::GlobalUserList,
        Self::TunneledMessage,
        Self::PrivilegedUsers,
        Self::HaveNoParent,
        Self::ParentIp,
        Self::ParentMinSpeed,
        Self::ParentSpeedRatio,
        Self::ParentInactivityTimeout,
        Self::SearchInactivityTimeout,
        Self::MinParentsInCache,
        Self::DistribPingInterval,
        Self::AddToPrivileged,
        Self::CheckPrivileges,
        Self::EmbeddedMessage,
        Self::AcceptChildren,
        Self::PossibleParents,
        Self::WishlistSearch,
        Self::WishlistInterval,
        Self::SimilarUsers,
        Self::ItemRecommendations,
        Self::ItemSimilarUsers,
        Self::RoomTickers,
        Self::RoomTickerAdded,
        Self::RoomTickerRemoved,
        Self::SetRoomTicker,
        Self::AddThingIHate,
        Self::RemoveThingIHate,
        Self::RoomSearch,
        Self::SendUploadSpeed,
        Self::UserPrivileged,
        Self::GivePrivileges,
        Self::NotifyPrivileges,
        Self::AckNotifyPrivileges,
        Self::BranchLevel,
        Self::BranchRoot,
        Self::ChildDepth,
        Self::ResetDistributed,
        Self::RoomMembers,
        Self::AddRoomMember,
        Self::RemoveRoomMember,
        Self::CancelRoomMembership,
        Self::CancelRoomOwnership,
        Self::RoomSomething,
        Self::RoomMembershipGranted,
        Self::RoomMembershipRevoked,
        Self::EnableRoomInvitations,
        Self::ChangePassword,
        Self::AddRoomOperator,
        Self::RemoveRoomOperator,
        Self::RoomOperatorshipGranted,
        Self::RoomOperatorshipRevoked,
        Self::RoomOperators,
        Self::MessageUsers,
        Self::JoinGlobalRoom,
        Self::LeaveGlobalRoom,
        Self::GlobalRoomMessage,
        Self::RelatedSearch,
        Self::ExcludedSearchPhrases,
        Self::CantConnectToPeer,
        Self::CantCreateRoom,
        Self::CantJoinRoom,
    ];

    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self as u32
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Login => "Login",
            Self::SetWaitPort => "SetWaitPort",
            Self::GetPeerAddress => "GetPeerAddress",
            Self::WatchUser => "WatchUser",
            Self::UnwatchUser => "UnwatchUser",
            Self::GetUserStatus => "GetUserStatus",
            Self::IgnoreUser => "IgnoreUser",
            Self::UnignoreUser => "UnignoreUser",
            Self::SayChatroom => "SayChatroom",
            Self::JoinRoom => "JoinRoom",
            Self::LeaveRoom => "LeaveRoom",
            Self::UserJoinedRoom => "UserJoinedRoom",
            Self::UserLeftRoom => "UserLeftRoom",
            Self::ConnectToPeer => "ConnectToPeer",
            Self::MessageUser => "MessageUser",
            Self::MessageAcked => "MessageAcked",
            Self::FileSearchRoom => "FileSearchRoom",
            Self::FileSearch => "FileSearch",
            Self::SetStatus => "SetStatus",
            Self::ServerPing => "ServerPing",
            Self::SendConnectToken => "SendConnectToken",
            Self::SendDownloadSpeed => "SendDownloadSpeed",
            Self::SharedFoldersFiles => "SharedFoldersFiles",
            Self::GetUserStats => "GetUserStats",
            Self::UploadSlotsFull => "UploadSlotsFull",
            Self::Relogged => "Relogged",
            Self::UserSearch => "UserSearch",
            Self::SimilarRecommendations => "SimilarRecommendations",
            Self::AddThingILike => "AddThingILike",
            Self::RemoveThingILike => "RemoveThingILike",
            Self::Recommendations => "Recommendations",
            Self::MyRecommendations => "MyRecommendations",
            Self::GlobalRecommendations => "GlobalRecommendations",
            Self::UserInterests => "UserInterests",
            Self::AdminCommand => "AdminCommand",
            Self::PlaceInLineRequest => "PlaceInLineRequest",
            Self::PlaceInLineResponse => "PlaceInLineResponse",
            Self::RoomAdded => "RoomAdded",
            Self::RoomRemoved => "RoomRemoved",
            Self::RoomList => "RoomList",
            Self::ExactFileSearch => "ExactFileSearch",
            Self::AdminMessage => "AdminMessage",
            Self::GlobalUserList => "GlobalUserList",
            Self::TunneledMessage => "TunneledMessage",
            Self::PrivilegedUsers => "PrivilegedUsers",
            Self::HaveNoParent => "HaveNoParent",
            Self::ParentIp => "ParentIp",
            Self::ParentMinSpeed => "ParentMinSpeed",
            Self::ParentSpeedRatio => "ParentSpeedRatio",
            Self::ParentInactivityTimeout => "ParentInactivityTimeout",
            Self::SearchInactivityTimeout => "SearchInactivityTimeout",
            Self::MinParentsInCache => "MinParentsInCache",
            Self::DistribPingInterval => "DistribPingInterval",
            Self::AddToPrivileged => "AddToPrivileged",
            Self::CheckPrivileges => "CheckPrivileges",
            Self::EmbeddedMessage => "EmbeddedMessage",
            Self::AcceptChildren => "AcceptChildren",
            Self::PossibleParents => "PossibleParents",
            Self::WishlistSearch => "WishlistSearch",
            Self::WishlistInterval => "WishlistInterval",
            Self::SimilarUsers => "SimilarUsers",
            Self::ItemRecommendations => "ItemRecommendations",
            Self::ItemSimilarUsers => "ItemSimilarUsers",
            Self::RoomTickers => "RoomTickers",
            Self::RoomTickerAdded => "RoomTickerAdded",
            Self::RoomTickerRemoved => "RoomTickerRemoved",
            Self::SetRoomTicker => "SetRoomTicker",
            Self::AddThingIHate => "AddThingIHate",
            Self::RemoveThingIHate => "RemoveThingIHate",
            Self::RoomSearch => "RoomSearch",
            Self::SendUploadSpeed => "SendUploadSpeed",
            Self::UserPrivileged => "UserPrivileged",
            Self::GivePrivileges => "GivePrivileges",
            Self::NotifyPrivileges => "NotifyPrivileges",
            Self::AckNotifyPrivileges => "AckNotifyPrivileges",
            Self::BranchLevel => "BranchLevel",
            Self::BranchRoot => "BranchRoot",
            Self::ChildDepth => "ChildDepth",
            Self::ResetDistributed => "ResetDistributed",
            Self::RoomMembers => "RoomMembers",
            Self::AddRoomMember => "AddRoomMember",
            Self::RemoveRoomMember => "RemoveRoomMember",
            Self::CancelRoomMembership => "CancelRoomMembership",
            Self::CancelRoomOwnership => "CancelRoomOwnership",
            Self::RoomSomething => "RoomSomething",
            Self::RoomMembershipGranted => "RoomMembershipGranted",
            Self::RoomMembershipRevoked => "RoomMembershipRevoked",
            Self::EnableRoomInvitations => "EnableRoomInvitations",
            Self::ChangePassword => "ChangePassword",
            Self::AddRoomOperator => "AddRoomOperator",
            Self::RemoveRoomOperator => "RemoveRoomOperator",
            Self::RoomOperatorshipGranted => "RoomOperatorshipGranted",
            Self::RoomOperatorshipRevoked => "RoomOperatorshipRevoked",
            Self::RoomOperators => "RoomOperators",
            Self::MessageUsers => "MessageUsers",
            Self::JoinGlobalRoom => "JoinGlobalRoom",
            Self::LeaveGlobalRoom => "LeaveGlobalRoom",
            Self::GlobalRoomMessage => "GlobalRoomMessage",
            Self::RelatedSearch => "RelatedSearch",
            Self::ExcludedSearchPhrases => "ExcludedSearchPhrases",
            Self::CantConnectToPeer => "CantConnectToPeer",
            Self::CantCreateRoom => "CantCreateRoom",
            Self::CantJoinRoom => "CantJoinRoom",
        }
    }
}

impl TryFrom<u32> for ServerCode {
    type Error = u32;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        let code = match value {
            1 => Self::Login,
            2 => Self::SetWaitPort,
            3 => Self::GetPeerAddress,
            5 => Self::WatchUser,
            6 => Self::UnwatchUser,
            7 => Self::GetUserStatus,
            11 => Self::IgnoreUser,
            12 => Self::UnignoreUser,
            13 => Self::SayChatroom,
            14 => Self::JoinRoom,
            15 => Self::LeaveRoom,
            16 => Self::UserJoinedRoom,
            17 => Self::UserLeftRoom,
            18 => Self::ConnectToPeer,
            22 => Self::MessageUser,
            23 => Self::MessageAcked,
            25 => Self::FileSearchRoom,
            26 => Self::FileSearch,
            28 => Self::SetStatus,
            32 => Self::ServerPing,
            33 => Self::SendConnectToken,
            34 => Self::SendDownloadSpeed,
            35 => Self::SharedFoldersFiles,
            36 => Self::GetUserStats,
            40 => Self::UploadSlotsFull,
            41 => Self::Relogged,
            42 => Self::UserSearch,
            50 => Self::SimilarRecommendations,
            51 => Self::AddThingILike,
            52 => Self::RemoveThingILike,
            54 => Self::Recommendations,
            55 => Self::MyRecommendations,
            56 => Self::GlobalRecommendations,
            57 => Self::UserInterests,
            58 => Self::AdminCommand,
            59 => Self::PlaceInLineRequest,
            60 => Self::PlaceInLineResponse,
            62 => Self::RoomAdded,
            63 => Self::RoomRemoved,
            64 => Self::RoomList,
            65 => Self::ExactFileSearch,
            66 => Self::AdminMessage,
            67 => Self::GlobalUserList,
            68 => Self::TunneledMessage,
            69 => Self::PrivilegedUsers,
            71 => Self::HaveNoParent,
            73 => Self::ParentIp,
            83 => Self::ParentMinSpeed,
            84 => Self::ParentSpeedRatio,
            86 => Self::ParentInactivityTimeout,
            87 => Self::SearchInactivityTimeout,
            88 => Self::MinParentsInCache,
            90 => Self::DistribPingInterval,
            91 => Self::AddToPrivileged,
            92 => Self::CheckPrivileges,
            93 => Self::EmbeddedMessage,
            100 => Self::AcceptChildren,
            102 => Self::PossibleParents,
            103 => Self::WishlistSearch,
            104 => Self::WishlistInterval,
            110 => Self::SimilarUsers,
            111 => Self::ItemRecommendations,
            112 => Self::ItemSimilarUsers,
            113 => Self::RoomTickers,
            114 => Self::RoomTickerAdded,
            115 => Self::RoomTickerRemoved,
            116 => Self::SetRoomTicker,
            117 => Self::AddThingIHate,
            118 => Self::RemoveThingIHate,
            120 => Self::RoomSearch,
            121 => Self::SendUploadSpeed,
            122 => Self::UserPrivileged,
            123 => Self::GivePrivileges,
            124 => Self::NotifyPrivileges,
            125 => Self::AckNotifyPrivileges,
            126 => Self::BranchLevel,
            127 => Self::BranchRoot,
            129 => Self::ChildDepth,
            130 => Self::ResetDistributed,
            133 => Self::RoomMembers,
            134 => Self::AddRoomMember,
            135 => Self::RemoveRoomMember,
            136 => Self::CancelRoomMembership,
            137 => Self::CancelRoomOwnership,
            138 => Self::RoomSomething,
            139 => Self::RoomMembershipGranted,
            140 => Self::RoomMembershipRevoked,
            141 => Self::EnableRoomInvitations,
            142 => Self::ChangePassword,
            143 => Self::AddRoomOperator,
            144 => Self::RemoveRoomOperator,
            145 => Self::RoomOperatorshipGranted,
            146 => Self::RoomOperatorshipRevoked,
            148 => Self::RoomOperators,
            149 => Self::MessageUsers,
            150 => Self::JoinGlobalRoom,
            151 => Self::LeaveGlobalRoom,
            152 => Self::GlobalRoomMessage,
            153 => Self::RelatedSearch,
            160 => Self::ExcludedSearchPhrases,
            1001 => Self::CantConnectToPeer,
            1002 => Self::CantCreateRoom,
            1003 => Self::CantJoinRoom,
            _ => return Err(value),
        };
        Ok(code)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub major_version: u32,
    pub hash: String,
    pub minor_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginResponse {
    Success {
        greet: String,
        ip: Ipv4Addr,
        hash: String,
        is_supporter: bool,
    },
    Failure {
        reason: String,
        detail: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitPort {
    pub port: u32,
    pub obfuscation: Option<ObfuscatedPort>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObfuscatedPort {
    pub kind: u32,
    pub port: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerAddress {
    pub username: String,
    pub ip: Ipv4Addr,
    pub port: u32,
    pub obfuscation_type: u32,
    pub obfuscated_port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectToPeerRequest {
    pub token: u32,
    pub username: String,
    pub connection_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectToPeerResponse {
    pub username: String,
    pub connection_type: String,
    pub ip: Ipv4Addr,
    pub port: u32,
    pub token: u32,
    pub privileged: bool,
    pub obfuscation_type: u32,
    pub obfuscated_port: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateMessage {
    pub id: u32,
    pub timestamp: u32,
    pub username: String,
    pub message: String,
    pub is_new: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchRequest {
    pub token: u32,
    pub query: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetedSearchRequest {
    pub target: String,
    pub token: u32,
    pub query: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PossibleParent {
    pub username: String,
    pub ip: Ipv4Addr,
    pub port: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserStats {
    pub average_speed: u32,
    pub upload_count: u32,
    pub unknown: u32,
    pub file_count: u32,
    pub directory_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchedUser {
    pub username: String,
    pub exists: bool,
    pub status: Option<u32>,
    pub stats: Option<UserStats>,
    pub country_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserStatus {
    pub username: String,
    pub status: u32,
    pub privileged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomListEntry {
    pub name: String,
    pub user_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomList {
    pub public_rooms: Vec<RoomListEntry>,
    pub owned_private_rooms: Vec<RoomListEntry>,
    pub private_rooms: Vec<RoomListEntry>,
    pub operated_private_rooms: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomUser {
    pub username: String,
    pub status: u32,
    pub average_speed: u32,
    pub upload_count: u64,
    pub file_count: u32,
    pub directory_count: u32,
    pub slots_free: u32,
    pub country_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinedRoom {
    pub room: String,
    pub users: Vec<RoomUser>,
    pub owner: Option<String>,
    pub operators: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerMessage {
    LoginRequest(LoginRequest),
    LoginResponse(LoginResponse),
    SetWaitPort(WaitPort),
    GetPeerAddressRequest {
        username: String,
    },
    GetPeerAddressResponse(PeerAddress),
    WatchUserRequest {
        username: String,
    },
    WatchUserResponse(WatchedUser),
    UnwatchUser {
        username: String,
    },
    GetUserStatusRequest {
        username: String,
    },
    GetUserStatusResponse(UserStatus),
    IgnoreUser {
        username: String,
    },
    UnignoreUser {
        username: String,
    },
    SayChatroomRequest {
        room: String,
        message: String,
    },
    SayChatroomResponse {
        room: String,
        username: String,
        message: String,
    },
    ConnectToPeerRequest(ConnectToPeerRequest),
    ConnectToPeerResponse(ConnectToPeerResponse),
    MessageUserRequest {
        username: String,
        message: String,
    },
    MessageUserResponse(PrivateMessage),
    MessageAcked {
        id: u32,
    },
    FileSearchRequest(SearchRequest),
    FileSearchIncoming {
        username: String,
        token: u32,
        query: String,
    },
    JoinRoom {
        room: String,
        private: bool,
    },
    JoinedRoom(JoinedRoom),
    LeaveRoom {
        room: String,
    },
    SetStatus {
        status: u32,
    },
    ServerPing,
    SharedFoldersFiles {
        folders: u32,
        files: u32,
    },
    GetUserStatsRequest {
        username: String,
    },
    GetUserStats {
        username: String,
        stats: UserStats,
    },
    Relogged,
    UserSearch(TargetedSearchRequest),
    AddThingILike {
        item: String,
    },
    RemoveThingILike {
        item: String,
    },
    RoomListRequest,
    RoomList(RoomList),
    PrivilegedUsers(Vec<String>),
    HaveNoParent {
        no_parent: bool,
    },
    ParentMinSpeed {
        speed: u32,
    },
    ParentSpeedRatio {
        ratio: u32,
    },
    CheckPrivilegesRequest,
    CheckPrivilegesResponse {
        seconds: u32,
    },
    AcceptChildren {
        accept: bool,
    },
    PossibleParents(Vec<PossibleParent>),
    WishlistSearch(SearchRequest),
    WishlistInterval {
        seconds: u32,
    },
    RoomSearch(TargetedSearchRequest),
    SendUploadSpeed {
        speed: u32,
    },
    BranchLevel {
        level: u32,
    },
    BranchRoot {
        username: String,
    },
    ResetDistributed,
    MessageUsers {
        usernames: Vec<String>,
        message: String,
    },
    JoinGlobalRoom,
    LeaveGlobalRoom,
    GlobalRoomMessage {
        room: String,
        username: String,
        message: String,
    },
    ExcludedSearchPhrases(Vec<String>),
    AddThingIHate {
        item: String,
    },
    RemoveThingIHate {
        item: String,
    },
    CantConnectToPeerRequest {
        token: u32,
        username: String,
    },
    CantConnectToPeerResponse {
        token: u32,
    },
    CantCreateRoom {
        room: String,
    },
    CantJoinRoom {
        room: String,
    },
    Unknown {
        code: u32,
        payload: Vec<u8>,
    },
}

impl ServerMessage {
    pub fn decode(frame: MessageFrame, direction: Direction) -> Result<Self, DecodeError> {
        let Ok(code) = ServerCode::try_from(frame.code) else {
            return Ok(Self::Unknown {
                code: frame.code,
                payload: frame.payload,
            });
        };

        let mut reader = Reader::new(&frame.payload);
        let message = match (code, direction) {
            (ServerCode::Login, Direction::ClientToServer) => {
                Self::LoginRequest(decode_login_request(&mut reader)?)
            }
            (ServerCode::Login, Direction::ServerToClient) => {
                Self::LoginResponse(decode_login_response(&mut reader)?)
            }
            (ServerCode::SetWaitPort, Direction::ClientToServer) => {
                Self::SetWaitPort(decode_wait_port(&mut reader)?)
            }
            (ServerCode::GetPeerAddress, Direction::ClientToServer) => {
                Self::GetPeerAddressRequest {
                    username: reader.read_string()?,
                }
            }
            (ServerCode::GetPeerAddress, Direction::ServerToClient) => {
                Self::GetPeerAddressResponse(decode_peer_address(&mut reader)?)
            }
            (ServerCode::WatchUser, Direction::ClientToServer) => Self::WatchUserRequest {
                username: reader.read_string()?,
            },
            (ServerCode::WatchUser, Direction::ServerToClient) => {
                Self::WatchUserResponse(decode_watched_user(&mut reader)?)
            }
            (ServerCode::UnwatchUser, _) => Self::UnwatchUser {
                username: reader.read_string()?,
            },
            (ServerCode::GetUserStatus, Direction::ClientToServer) => Self::GetUserStatusRequest {
                username: reader.read_string()?,
            },
            (ServerCode::GetUserStatus, Direction::ServerToClient) => {
                Self::GetUserStatusResponse(UserStatus {
                    username: reader.read_string()?,
                    status: reader.read_u32_le()?,
                    privileged: reader.read_bool()?,
                })
            }
            (ServerCode::IgnoreUser, _) => Self::IgnoreUser {
                username: reader.read_string()?,
            },
            (ServerCode::UnignoreUser, _) => Self::UnignoreUser {
                username: reader.read_string()?,
            },
            (ServerCode::SayChatroom, Direction::ClientToServer) => Self::SayChatroomRequest {
                room: reader.read_string()?,
                message: reader.read_string()?,
            },
            (ServerCode::SayChatroom, Direction::ServerToClient) => Self::SayChatroomResponse {
                room: reader.read_string()?,
                username: reader.read_string()?,
                message: reader.read_string()?,
            },
            (ServerCode::ConnectToPeer, Direction::ClientToServer) => {
                Self::ConnectToPeerRequest(ConnectToPeerRequest {
                    token: reader.read_u32_le()?,
                    username: reader.read_string()?,
                    connection_type: reader.read_string()?,
                })
            }
            (ServerCode::ConnectToPeer, Direction::ServerToClient) => {
                let username = reader.read_string()?;
                let connection_type = reader.read_string()?;
                let ip = reader.read_ipv4()?;
                let port = reader.read_u32_le()?;
                let token = reader.read_u32_le()?;
                let privileged = reader.read_bool()?;
                let (obfuscation_type, obfuscated_port) = if reader.is_empty() {
                    (0, 0)
                } else {
                    normalize_optional_obfuscated_port(reader.read_u32_le()?, reader.read_u32_le()?)
                };
                Self::ConnectToPeerResponse(ConnectToPeerResponse {
                    username,
                    connection_type,
                    ip,
                    port,
                    token,
                    privileged,
                    obfuscation_type,
                    obfuscated_port,
                })
            }
            (ServerCode::MessageUser, Direction::ClientToServer) => Self::MessageUserRequest {
                username: reader.read_string()?,
                message: reader.read_string()?,
            },
            (ServerCode::MessageUser, Direction::ServerToClient) => {
                Self::MessageUserResponse(PrivateMessage {
                    id: reader.read_u32_le()?,
                    timestamp: reader.read_u32_le()?,
                    username: reader.read_string()?,
                    message: reader.read_string()?,
                    is_new: reader.read_bool()?,
                })
            }
            (ServerCode::MessageAcked, Direction::ClientToServer) => Self::MessageAcked {
                id: reader.read_u32_le()?,
            },
            (ServerCode::FileSearch, Direction::ClientToServer) => {
                Self::FileSearchRequest(decode_search_request(&mut reader)?)
            }
            (ServerCode::FileSearch, Direction::ServerToClient) => Self::FileSearchIncoming {
                username: reader.read_string()?,
                token: reader.read_u32_le()?,
                query: reader.read_string()?,
            },
            (ServerCode::JoinRoom, Direction::ClientToServer) => Self::JoinRoom {
                room: reader.read_string()?,
                private: if reader.is_empty() {
                    false
                } else {
                    reader.read_u32_le()? != 0
                },
            },
            (ServerCode::JoinRoom, Direction::ServerToClient) => {
                Self::JoinedRoom(decode_joined_room(&mut reader)?)
            }
            (ServerCode::LeaveRoom, _) => Self::LeaveRoom {
                room: reader.read_string()?,
            },
            (ServerCode::SetStatus, Direction::ClientToServer) => Self::SetStatus {
                status: reader.read_u32_le()?,
            },
            (ServerCode::ServerPing, _) => Self::ServerPing,
            (ServerCode::SharedFoldersFiles, Direction::ClientToServer) => {
                Self::SharedFoldersFiles {
                    folders: reader.read_u32_le()?,
                    files: reader.read_u32_le()?,
                }
            }
            (ServerCode::GetUserStats, Direction::ClientToServer) => Self::GetUserStatsRequest {
                username: reader.read_string()?,
            },
            (ServerCode::GetUserStats, Direction::ServerToClient) => Self::GetUserStats {
                username: reader.read_string()?,
                stats: decode_user_stats(&mut reader)?,
            },
            (ServerCode::Relogged, Direction::ServerToClient) => Self::Relogged,
            (ServerCode::UserSearch, Direction::ClientToServer) => {
                Self::UserSearch(decode_targeted_search_request(&mut reader)?)
            }
            (ServerCode::AddThingILike, Direction::ClientToServer) => Self::AddThingILike {
                item: reader.read_string()?,
            },
            (ServerCode::RemoveThingILike, Direction::ClientToServer) => Self::RemoveThingILike {
                item: reader.read_string()?,
            },
            (ServerCode::RoomList, Direction::ClientToServer) => Self::RoomListRequest,
            (ServerCode::RoomList, Direction::ServerToClient) => {
                Self::RoomList(decode_room_list(&mut reader)?)
            }
            (ServerCode::PrivilegedUsers, Direction::ServerToClient) => {
                Self::PrivilegedUsers(decode_string_vec(&mut reader)?)
            }
            (ServerCode::HaveNoParent, Direction::ClientToServer) => Self::HaveNoParent {
                no_parent: reader.read_bool()?,
            },
            (ServerCode::ParentMinSpeed, Direction::ServerToClient) => Self::ParentMinSpeed {
                speed: reader.read_u32_le()?,
            },
            (ServerCode::ParentSpeedRatio, Direction::ServerToClient) => Self::ParentSpeedRatio {
                ratio: reader.read_u32_le()?,
            },
            (ServerCode::CheckPrivileges, Direction::ClientToServer) => {
                Self::CheckPrivilegesRequest
            }
            (ServerCode::CheckPrivileges, Direction::ServerToClient) => {
                Self::CheckPrivilegesResponse {
                    seconds: reader.read_u32_le()?,
                }
            }
            (ServerCode::AcceptChildren, Direction::ClientToServer) => Self::AcceptChildren {
                accept: reader.read_bool()?,
            },
            (ServerCode::PossibleParents, Direction::ServerToClient) => {
                Self::PossibleParents(decode_possible_parents(&mut reader)?)
            }
            (ServerCode::WishlistSearch, Direction::ClientToServer) => {
                Self::WishlistSearch(decode_search_request(&mut reader)?)
            }
            (ServerCode::WishlistInterval, Direction::ServerToClient) => Self::WishlistInterval {
                seconds: reader.read_u32_le()?,
            },
            (ServerCode::RoomSearch, Direction::ClientToServer) => {
                Self::RoomSearch(decode_targeted_search_request(&mut reader)?)
            }
            (ServerCode::SendUploadSpeed, Direction::ClientToServer) => Self::SendUploadSpeed {
                speed: reader.read_u32_le()?,
            },
            (ServerCode::BranchLevel, Direction::ClientToServer) => Self::BranchLevel {
                level: reader.read_u32_le()?,
            },
            (ServerCode::BranchRoot, Direction::ClientToServer) => Self::BranchRoot {
                username: reader.read_string()?,
            },
            (ServerCode::ResetDistributed, Direction::ServerToClient) => Self::ResetDistributed,
            (ServerCode::MessageUsers, Direction::ClientToServer) => Self::MessageUsers {
                usernames: decode_string_vec(&mut reader)?,
                message: reader.read_string()?,
            },
            (ServerCode::JoinGlobalRoom, Direction::ClientToServer) => Self::JoinGlobalRoom,
            (ServerCode::LeaveGlobalRoom, Direction::ClientToServer) => Self::LeaveGlobalRoom,
            (ServerCode::GlobalRoomMessage, Direction::ServerToClient) => Self::GlobalRoomMessage {
                room: reader.read_string()?,
                username: reader.read_string()?,
                message: reader.read_string()?,
            },
            (ServerCode::ExcludedSearchPhrases, Direction::ServerToClient) => {
                Self::ExcludedSearchPhrases(decode_string_vec(&mut reader)?)
            }
            (ServerCode::AddThingIHate, Direction::ClientToServer) => Self::AddThingIHate {
                item: reader.read_string()?,
            },
            (ServerCode::RemoveThingIHate, Direction::ClientToServer) => Self::RemoveThingIHate {
                item: reader.read_string()?,
            },
            (ServerCode::CantConnectToPeer, Direction::ClientToServer) => {
                Self::CantConnectToPeerRequest {
                    token: reader.read_u32_le()?,
                    username: reader.read_string()?,
                }
            }
            (ServerCode::CantConnectToPeer, Direction::ServerToClient) => {
                Self::CantConnectToPeerResponse {
                    token: reader.read_u32_le()?,
                }
            }
            (ServerCode::CantCreateRoom, Direction::ServerToClient) => Self::CantCreateRoom {
                room: reader.read_string()?,
            },
            (ServerCode::CantJoinRoom, Direction::ServerToClient) => Self::CantJoinRoom {
                room: reader.read_string()?,
            },
            _ => {
                return Ok(Self::Unknown {
                    code: code.as_u32(),
                    payload: frame.payload,
                });
            }
        };

        reader.finish()?;
        Ok(message)
    }

    pub fn encode(&self) -> Result<MessageFrame, EncodeError> {
        let mut writer = Writer::new();
        let code = match self {
            Self::LoginRequest(value) => {
                writer.write_string(&value.username)?;
                writer.write_string(&value.password)?;
                writer.write_u32_le(value.major_version);
                writer.write_string(&value.hash)?;
                writer.write_u32_le(value.minor_version);
                ServerCode::Login
            }
            Self::LoginResponse(value) => {
                match value {
                    LoginResponse::Success {
                        greet,
                        ip,
                        hash,
                        is_supporter,
                    } => {
                        writer.write_bool(true);
                        writer.write_string(greet)?;
                        writer.write_ipv4(*ip);
                        writer.write_string(hash)?;
                        writer.write_bool(*is_supporter);
                    }
                    LoginResponse::Failure { reason, detail } => {
                        writer.write_bool(false);
                        writer.write_string(reason)?;
                        if let Some(detail) = detail {
                            writer.write_string(detail)?;
                        }
                    }
                }
                ServerCode::Login
            }
            Self::SetWaitPort(value) => {
                writer.write_u32_le(value.port);
                if let Some(obfuscation) = &value.obfuscation {
                    writer.write_u32_le(obfuscation.kind);
                    writer.write_u32_le(obfuscation.port);
                }
                ServerCode::SetWaitPort
            }
            Self::GetPeerAddressRequest { username } => {
                writer.write_string(username)?;
                ServerCode::GetPeerAddress
            }
            Self::GetPeerAddressResponse(value) => {
                writer.write_string(&value.username)?;
                writer.write_ipv4(value.ip);
                writer.write_u32_le(value.port);
                writer.write_u32_le(value.obfuscation_type);
                writer.write_u16_le(value.obfuscated_port);
                ServerCode::GetPeerAddress
            }
            Self::WatchUserRequest { username } => {
                writer.write_string(username)?;
                ServerCode::WatchUser
            }
            Self::WatchUserResponse(value) => {
                writer.write_string(&value.username)?;
                writer.write_bool(value.exists);
                if value.exists {
                    writer.write_u32_le(value.status.unwrap_or_default());
                    if let Some(stats) = &value.stats {
                        encode_user_stats(&mut writer, stats);
                    }
                    if let Some(country_code) = &value.country_code {
                        writer.write_string(country_code)?;
                    }
                }
                ServerCode::WatchUser
            }
            Self::UnwatchUser { username } => {
                writer.write_string(username)?;
                ServerCode::UnwatchUser
            }
            Self::GetUserStatusRequest { username } => {
                writer.write_string(username)?;
                ServerCode::GetUserStatus
            }
            Self::GetUserStatusResponse(value) => {
                writer.write_string(&value.username)?;
                writer.write_u32_le(value.status);
                writer.write_bool(value.privileged);
                ServerCode::GetUserStatus
            }
            Self::IgnoreUser { username } => {
                writer.write_string(username)?;
                ServerCode::IgnoreUser
            }
            Self::UnignoreUser { username } => {
                writer.write_string(username)?;
                ServerCode::UnignoreUser
            }
            Self::SayChatroomRequest { room, message } => {
                writer.write_string(room)?;
                writer.write_string(message)?;
                ServerCode::SayChatroom
            }
            Self::SayChatroomResponse {
                room,
                username,
                message,
            } => {
                writer.write_string(room)?;
                writer.write_string(username)?;
                writer.write_string(message)?;
                ServerCode::SayChatroom
            }
            Self::ConnectToPeerRequest(value) => {
                writer.write_u32_le(value.token);
                writer.write_string(&value.username)?;
                writer.write_string(&value.connection_type)?;
                ServerCode::ConnectToPeer
            }
            Self::ConnectToPeerResponse(value) => {
                writer.write_string(&value.username)?;
                writer.write_string(&value.connection_type)?;
                writer.write_ipv4(value.ip);
                writer.write_u32_le(value.port);
                writer.write_u32_le(value.token);
                writer.write_bool(value.privileged);
                writer.write_u32_le(value.obfuscation_type);
                writer.write_u32_le(value.obfuscated_port);
                ServerCode::ConnectToPeer
            }
            Self::MessageUserRequest { username, message } => {
                writer.write_string(username)?;
                writer.write_string(message)?;
                ServerCode::MessageUser
            }
            Self::MessageUserResponse(value) => {
                writer.write_u32_le(value.id);
                writer.write_u32_le(value.timestamp);
                writer.write_string(&value.username)?;
                writer.write_string(&value.message)?;
                writer.write_bool(value.is_new);
                ServerCode::MessageUser
            }
            Self::MessageAcked { id } => {
                writer.write_u32_le(*id);
                ServerCode::MessageAcked
            }
            Self::FileSearchRequest(value) => {
                encode_search_request(&mut writer, value)?;
                ServerCode::FileSearch
            }
            Self::FileSearchIncoming {
                username,
                token,
                query,
            } => {
                writer.write_string(username)?;
                writer.write_u32_le(*token);
                writer.write_string(query)?;
                ServerCode::FileSearch
            }
            Self::JoinRoom { room, private } => {
                writer.write_string(room)?;
                writer.write_u32_le(u32::from(*private));
                ServerCode::JoinRoom
            }
            Self::JoinedRoom(value) => {
                encode_joined_room(&mut writer, value)?;
                ServerCode::JoinRoom
            }
            Self::LeaveRoom { room } => {
                writer.write_string(room)?;
                ServerCode::LeaveRoom
            }
            Self::SetStatus { status } => {
                writer.write_u32_le(*status);
                ServerCode::SetStatus
            }
            Self::ServerPing => ServerCode::ServerPing,
            Self::SharedFoldersFiles { folders, files } => {
                writer.write_u32_le(*folders);
                writer.write_u32_le(*files);
                ServerCode::SharedFoldersFiles
            }
            Self::GetUserStatsRequest { username } => {
                writer.write_string(username)?;
                ServerCode::GetUserStats
            }
            Self::GetUserStats { username, stats } => {
                writer.write_string(username)?;
                encode_user_stats(&mut writer, stats);
                ServerCode::GetUserStats
            }
            Self::Relogged => ServerCode::Relogged,
            Self::UserSearch(value) => {
                encode_targeted_search_request(&mut writer, value)?;
                ServerCode::UserSearch
            }
            Self::AddThingILike { item } => {
                writer.write_string(item)?;
                ServerCode::AddThingILike
            }
            Self::RemoveThingILike { item } => {
                writer.write_string(item)?;
                ServerCode::RemoveThingILike
            }
            Self::RoomListRequest => ServerCode::RoomList,
            Self::RoomList(value) => {
                encode_room_list(&mut writer, value)?;
                ServerCode::RoomList
            }
            Self::PrivilegedUsers(value) => {
                encode_string_vec(&mut writer, value)?;
                ServerCode::PrivilegedUsers
            }
            Self::HaveNoParent { no_parent } => {
                writer.write_bool(*no_parent);
                ServerCode::HaveNoParent
            }
            Self::ParentMinSpeed { speed } => {
                writer.write_u32_le(*speed);
                ServerCode::ParentMinSpeed
            }
            Self::ParentSpeedRatio { ratio } => {
                writer.write_u32_le(*ratio);
                ServerCode::ParentSpeedRatio
            }
            Self::CheckPrivilegesRequest => ServerCode::CheckPrivileges,
            Self::CheckPrivilegesResponse { seconds } => {
                writer.write_u32_le(*seconds);
                ServerCode::CheckPrivileges
            }
            Self::AcceptChildren { accept } => {
                writer.write_bool(*accept);
                ServerCode::AcceptChildren
            }
            Self::PossibleParents(value) => {
                encode_possible_parents(&mut writer, value)?;
                ServerCode::PossibleParents
            }
            Self::WishlistSearch(value) => {
                encode_search_request(&mut writer, value)?;
                ServerCode::WishlistSearch
            }
            Self::WishlistInterval { seconds } => {
                writer.write_u32_le(*seconds);
                ServerCode::WishlistInterval
            }
            Self::RoomSearch(value) => {
                encode_targeted_search_request(&mut writer, value)?;
                ServerCode::RoomSearch
            }
            Self::SendUploadSpeed { speed } => {
                writer.write_u32_le(*speed);
                ServerCode::SendUploadSpeed
            }
            Self::BranchLevel { level } => {
                writer.write_u32_le(*level);
                ServerCode::BranchLevel
            }
            Self::BranchRoot { username } => {
                writer.write_string(username)?;
                ServerCode::BranchRoot
            }
            Self::ResetDistributed => ServerCode::ResetDistributed,
            Self::MessageUsers { usernames, message } => {
                encode_string_vec(&mut writer, usernames)?;
                writer.write_string(message)?;
                ServerCode::MessageUsers
            }
            Self::JoinGlobalRoom => ServerCode::JoinGlobalRoom,
            Self::LeaveGlobalRoom => ServerCode::LeaveGlobalRoom,
            Self::GlobalRoomMessage {
                room,
                username,
                message,
            } => {
                writer.write_string(room)?;
                writer.write_string(username)?;
                writer.write_string(message)?;
                ServerCode::GlobalRoomMessage
            }
            Self::ExcludedSearchPhrases(value) => {
                encode_string_vec(&mut writer, value)?;
                ServerCode::ExcludedSearchPhrases
            }
            Self::AddThingIHate { item } => {
                writer.write_string(item)?;
                ServerCode::AddThingIHate
            }
            Self::RemoveThingIHate { item } => {
                writer.write_string(item)?;
                ServerCode::RemoveThingIHate
            }
            Self::CantConnectToPeerRequest { token, username } => {
                writer.write_u32_le(*token);
                writer.write_string(username)?;
                ServerCode::CantConnectToPeer
            }
            Self::CantConnectToPeerResponse { token } => {
                writer.write_u32_le(*token);
                ServerCode::CantConnectToPeer
            }
            Self::CantCreateRoom { room } => {
                writer.write_string(room)?;
                ServerCode::CantCreateRoom
            }
            Self::CantJoinRoom { room } => {
                writer.write_string(room)?;
                ServerCode::CantJoinRoom
            }
            Self::Unknown { code, payload } => {
                return Ok(MessageFrame::new(*code, payload.clone()))
            }
        };

        Ok(MessageFrame::new(code.as_u32(), writer.into_inner()))
    }
}

fn decode_login_request(reader: &mut Reader<'_>) -> Result<LoginRequest, DecodeError> {
    Ok(LoginRequest {
        username: reader.read_string()?,
        password: reader.read_string()?,
        major_version: reader.read_u32_le()?,
        hash: reader.read_string()?,
        minor_version: reader.read_u32_le()?,
    })
}

fn decode_login_response(reader: &mut Reader<'_>) -> Result<LoginResponse, DecodeError> {
    if reader.read_bool()? {
        Ok(LoginResponse::Success {
            greet: reader.read_string()?,
            ip: reader.read_ipv4()?,
            hash: reader.read_string()?,
            is_supporter: reader.read_bool()?,
        })
    } else {
        let reason = reader.read_string()?;
        let detail = if reader.is_empty() {
            None
        } else {
            Some(reader.read_string()?)
        };
        Ok(LoginResponse::Failure { reason, detail })
    }
}

fn decode_wait_port(reader: &mut Reader<'_>) -> Result<WaitPort, DecodeError> {
    let port = reader.read_u32_le()?;
    let obfuscation = if reader.is_empty() {
        None
    } else {
        Some(ObfuscatedPort {
            kind: reader.read_u32_le()?,
            port: reader.read_u32_le()?,
        })
    };
    Ok(WaitPort { port, obfuscation })
}

fn decode_peer_address(reader: &mut Reader<'_>) -> Result<PeerAddress, DecodeError> {
    let username = reader.read_string()?;
    let ip = reader.read_ipv4()?;
    let port = reader.read_u32_le()?;
    let (obfuscation_type, obfuscated_port) = if reader.is_empty() {
        (0, 0)
    } else {
        normalize_optional_obfuscated_port(reader.read_u32_le()?, u32::from(reader.read_u16_le()?))
    };
    Ok(PeerAddress {
        username,
        ip,
        port,
        obfuscation_type,
        obfuscated_port: u16::try_from(obfuscated_port)
            .expect("peer-address obfuscated port originated as u16"),
    })
}

fn normalize_optional_obfuscated_port(obfuscation_type: u32, port: u32) -> (u32, u32) {
    if obfuscation_type == 1 && !(1..=u32::from(u16::MAX)).contains(&port) {
        (0, 0)
    } else {
        (obfuscation_type, port)
    }
}

fn decode_watched_user(reader: &mut Reader<'_>) -> Result<WatchedUser, DecodeError> {
    let username = reader.read_string()?;
    let exists = reader.read_bool()?;
    if !exists {
        return Ok(WatchedUser {
            username,
            exists,
            status: None,
            stats: None,
            country_code: None,
        });
    }

    let status = reader.read_u32_le()?;
    let stats = decode_user_stats(reader)?;
    let country_code = if reader.is_empty() {
        None
    } else {
        Some(reader.read_string()?)
    };

    Ok(WatchedUser {
        username,
        exists,
        status: Some(status),
        stats: Some(stats),
        country_code,
    })
}

fn decode_user_stats(reader: &mut Reader<'_>) -> Result<UserStats, DecodeError> {
    Ok(UserStats {
        average_speed: reader.read_u32_le()?,
        upload_count: reader.read_u32_le()?,
        unknown: reader.read_u32_le()?,
        file_count: reader.read_u32_le()?,
        directory_count: reader.read_u32_le()?,
    })
}

fn encode_user_stats(writer: &mut Writer, value: &UserStats) {
    writer.write_u32_le(value.average_speed);
    writer.write_u32_le(value.upload_count);
    writer.write_u32_le(value.unknown);
    writer.write_u32_le(value.file_count);
    writer.write_u32_le(value.directory_count);
}

fn decode_search_request(reader: &mut Reader<'_>) -> Result<SearchRequest, DecodeError> {
    Ok(SearchRequest {
        token: reader.read_u32_le()?,
        query: reader.read_string()?,
    })
}

fn encode_search_request(writer: &mut Writer, value: &SearchRequest) -> Result<(), EncodeError> {
    writer.write_u32_le(value.token);
    writer.write_string(&value.query)
}

fn decode_targeted_search_request(
    reader: &mut Reader<'_>,
) -> Result<TargetedSearchRequest, DecodeError> {
    Ok(TargetedSearchRequest {
        target: reader.read_string()?,
        token: reader.read_u32_le()?,
        query: reader.read_string()?,
    })
}

fn encode_targeted_search_request(
    writer: &mut Writer,
    value: &TargetedSearchRequest,
) -> Result<(), EncodeError> {
    writer.write_string(&value.target)?;
    writer.write_u32_le(value.token);
    writer.write_string(&value.query)
}

fn decode_string_vec(reader: &mut Reader<'_>) -> Result<Vec<String>, DecodeError> {
    let count = reader.read_bounded_count("string vec", 4)?;
    let mut values = Vec::new();
    for _ in 0..count {
        values.push(reader.read_string()?);
    }
    Ok(values)
}

fn encode_string_vec(writer: &mut Writer, values: &[String]) -> Result<(), EncodeError> {
    let count = u32::try_from(values.len())
        .map_err(|_| EncodeError::length_overflow("string vec", values.len()))?;
    writer.write_u32_le(count);
    for value in values {
        writer.write_string(value)?;
    }
    Ok(())
}

fn matching_room_count(
    reader: &mut Reader<'_>,
    field: &'static str,
    expected: usize,
    minimum_bytes_per_item: usize,
) -> Result<usize, DecodeError> {
    let actual = reader.read_bounded_count(field, minimum_bytes_per_item)?;
    if actual != expected {
        return Err(DecodeError::InvalidVectorLength {
            field,
            expected,
            actual,
        });
    }
    Ok(actual)
}

fn decode_joined_room(reader: &mut Reader<'_>) -> Result<JoinedRoom, DecodeError> {
    let room = reader.read_string()?;
    let user_count = reader.read_bounded_count("room users", 4)?;
    let mut usernames = Vec::with_capacity(user_count);
    for _ in 0..user_count {
        usernames.push(reader.read_string()?);
    }

    matching_room_count(reader, "room user statuses", user_count, 4)?;
    let mut statuses = Vec::with_capacity(user_count);
    for _ in 0..user_count {
        statuses.push(reader.read_u32_le()?);
    }

    matching_room_count(reader, "room user data", user_count, 20)?;
    let mut data = Vec::with_capacity(user_count);
    for _ in 0..user_count {
        data.push((
            reader.read_u32_le()?,
            reader.read_u64_le()?,
            reader.read_u32_le()?,
            reader.read_u32_le()?,
        ));
    }

    matching_room_count(reader, "room user slots", user_count, 4)?;
    let mut slots = Vec::with_capacity(user_count);
    for _ in 0..user_count {
        slots.push(reader.read_u32_le()?);
    }

    matching_room_count(reader, "room user countries", user_count, 4)?;
    let mut countries = Vec::with_capacity(user_count);
    for _ in 0..user_count {
        countries.push(reader.read_string()?);
    }

    let mut users = Vec::with_capacity(user_count);
    for (index, username) in usernames.into_iter().enumerate() {
        let (average_speed, upload_count, file_count, directory_count) = data[index];
        users.push(RoomUser {
            username,
            status: statuses[index],
            average_speed,
            upload_count,
            file_count,
            directory_count,
            slots_free: slots[index],
            country_code: countries[index].clone(),
        });
    }

    let (owner, operators) = if reader.is_empty() {
        (None, Vec::new())
    } else {
        (Some(reader.read_string()?), decode_string_vec(reader)?)
    };
    Ok(JoinedRoom {
        room,
        users,
        owner,
        operators,
    })
}

fn encode_joined_room(writer: &mut Writer, value: &JoinedRoom) -> Result<(), EncodeError> {
    let count = u32::try_from(value.users.len())
        .map_err(|_| EncodeError::length_overflow("room users", value.users.len()))?;
    writer.write_string(&value.room)?;
    writer.write_u32_le(count);
    for user in &value.users {
        writer.write_string(&user.username)?;
    }
    writer.write_u32_le(count);
    for user in &value.users {
        writer.write_u32_le(user.status);
    }
    writer.write_u32_le(count);
    for user in &value.users {
        writer.write_u32_le(user.average_speed);
        writer.write_u64_le(user.upload_count);
        writer.write_u32_le(user.file_count);
        writer.write_u32_le(user.directory_count);
    }
    writer.write_u32_le(count);
    for user in &value.users {
        writer.write_u32_le(user.slots_free);
    }
    writer.write_u32_le(count);
    for user in &value.users {
        writer.write_string(&user.country_code)?;
    }
    if let Some(owner) = &value.owner {
        writer.write_string(owner)?;
        encode_string_vec(writer, &value.operators)?;
    }
    Ok(())
}

fn decode_room_list(reader: &mut Reader<'_>) -> Result<RoomList, DecodeError> {
    Ok(RoomList {
        public_rooms: decode_room_entries(reader)?,
        owned_private_rooms: decode_room_entries(reader)?,
        private_rooms: decode_room_entries(reader)?,
        operated_private_rooms: decode_string_vec(reader)?,
    })
}

fn encode_room_list(writer: &mut Writer, value: &RoomList) -> Result<(), EncodeError> {
    encode_room_entries(writer, &value.public_rooms)?;
    encode_room_entries(writer, &value.owned_private_rooms)?;
    encode_room_entries(writer, &value.private_rooms)?;
    encode_string_vec(writer, &value.operated_private_rooms)
}

fn decode_room_entries(reader: &mut Reader<'_>) -> Result<Vec<RoomListEntry>, DecodeError> {
    let names = decode_string_vec(reader)?;
    let counts_len = reader.read_u32_le()? as usize;
    if counts_len != names.len() {
        return Err(DecodeError::InvalidVectorLength {
            field: "room user counts",
            expected: names.len(),
            actual: counts_len,
        });
    }

    let mut entries = Vec::with_capacity(names.len());
    for name in names {
        entries.push(RoomListEntry {
            name,
            user_count: reader.read_u32_le()?,
        });
    }
    Ok(entries)
}

fn encode_room_entries(writer: &mut Writer, entries: &[RoomListEntry]) -> Result<(), EncodeError> {
    let count = u32::try_from(entries.len())
        .map_err(|_| EncodeError::length_overflow("room entries", entries.len()))?;
    writer.write_u32_le(count);
    for entry in entries {
        writer.write_string(&entry.name)?;
    }
    writer.write_u32_le(count);
    for entry in entries {
        writer.write_u32_le(entry.user_count);
    }
    Ok(())
}

fn decode_possible_parents(reader: &mut Reader<'_>) -> Result<Vec<PossibleParent>, DecodeError> {
    let count = reader.read_bounded_count("possible parents", 12)?;
    let mut parents = Vec::new();
    for _ in 0..count {
        parents.push(PossibleParent {
            username: reader.read_string()?,
            ip: reader.read_ipv4()?,
            port: reader.read_u32_le()?,
        });
    }
    Ok(parents)
}

fn encode_possible_parents(
    writer: &mut Writer,
    values: &[PossibleParent],
) -> Result<(), EncodeError> {
    let count = u32::try_from(values.len())
        .map_err(|_| EncodeError::length_overflow("possible parents", values.len()))?;
    writer.write_u32_le(count);
    for value in values {
        writer.write_string(&value.username)?;
        writer.write_ipv4(value.ip);
        writer.write_u32_le(value.port);
    }
    Ok(())
}
