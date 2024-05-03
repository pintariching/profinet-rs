/// Context Management protocol machine Device
///

pub enum CmdevState {
    PowerOn,
    ConnectInd,
    ConnectResp,
    CmsuConf,
    PrmEndInd,
    PrmEndResp,
    AppReady,
    AppReadyConf,
    WaitData,
    DataExchange,
    Abort,
}

impl CmdevState {}
