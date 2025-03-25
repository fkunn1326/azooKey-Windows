use shared::proto::{
    window_service_server::WindowService as WindowServiceProto, EmptyResponse, SetCandidateRequest,
    SetInputModeRequest, SetPositionRequest, SetSelectionRequest,
};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

#[derive(Debug, Clone)]
pub struct WindowController {
    sender: mpsc::Sender<WindowAction>,
}

impl WindowController {
    pub fn new(sender: mpsc::Sender<WindowAction>) -> Self {
        Self { sender }
    }
}

// ウィンドウ操作コマンド
#[derive(Debug, serde::Serialize)]
pub enum WindowAction {
    Show,
    Hide,
    SetPosition {
        top: i32,
        left: i32,
        bottom: i32,
        right: i32,
    },
    SetSelection {
        index: i32,
    },
    SetCandidate {
        candidates: Vec<String>,
    },
    SetInputMode(String),
}

#[derive(Debug)]
pub struct WindowService {
    pub controller: WindowController,
}

#[tonic::async_trait]
impl WindowServiceProto for WindowService {
    async fn show_window(
        &self,
        _request: Request<EmptyResponse>,
    ) -> Result<Response<EmptyResponse>, Status> {
        self.controller
            .sender
            .send(WindowAction::Show)
            .await
            .unwrap();
        Ok(Response::new(EmptyResponse {}))
    }

    async fn hide_window(
        &self,
        _request: Request<EmptyResponse>,
    ) -> Result<Response<EmptyResponse>, Status> {
        self.controller
            .sender
            .send(WindowAction::Hide)
            .await
            .unwrap();
        Ok(Response::new(EmptyResponse {}))
    }
    async fn set_window_position(
        &self,
        request: Request<SetPositionRequest>,
    ) -> Result<Response<EmptyResponse>, Status> {
        let position = request.into_inner().position.unwrap();
        let top = position.top;
        let left = position.left;
        let bottom = position.bottom;
        let right = position.right;
        self.controller
            .sender
            .send(WindowAction::SetPosition {
                top,
                left,
                bottom,
                right,
            })
            .await
            .unwrap();

        Ok(Response::new(EmptyResponse {}))
    }

    async fn set_candidate(
        &self,
        request: Request<SetCandidateRequest>,
    ) -> Result<Response<EmptyResponse>, Status> {
        let candidate = request.into_inner().candidates;

        self.controller
            .sender
            .send(WindowAction::SetCandidate {
                candidates: candidate,
            })
            .await
            .unwrap();

        Ok(Response::new(EmptyResponse {}))
    }

    async fn set_selection(
        &self,
        request: Request<SetSelectionRequest>,
    ) -> Result<Response<EmptyResponse>, Status> {
        let index = request.into_inner().index;
        self.controller
            .sender
            .send(WindowAction::SetSelection { index })
            .await
            .unwrap();

        Ok(Response::new(EmptyResponse {}))
    }

    async fn set_input_mode(
        &self,
        request: Request<SetInputModeRequest>,
    ) -> Result<Response<EmptyResponse>, Status> {
        let mode = request.into_inner().mode;
        self.controller
            .sender
            .send(WindowAction::SetInputMode(mode))
            .await
            .unwrap();

        Ok(Response::new(EmptyResponse {}))
    }
}
