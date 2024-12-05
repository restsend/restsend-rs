use std::sync::{Arc, Mutex};
use restsend_sdk::{models::Conversation, request::ChatRequest};
pub(super) type CallbackOnConnected = Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>;

#[flutter_rust_bridge::frb(ignore)]
pub(super) struct CallbackDartWrap {
    pub(super) cb_on_connected: CallbackOnConnected,
    // pub(super) cb_on_connecting: CallbackFunction,
    // pub(super) cb_on_token_expired: CallbackFunction,
    // pub(super) cb_on_net_broken: CallbackFunction,
    // pub(super) cb_on_kickoff_by_other_client: CallbackFunction,
    // pub(super) cb_on_system_request: CallbackFunction,
    // pub(super) cb_on_unknown_request: CallbackFunction,
    // pub(super) cb_on_topic_typing: CallbackFunction,
    // pub(super) cb_on_topic_message: CallbackFunction,
    // pub(super) cb_on_topic_read: CallbackFunction,
    // pub(super) cb_on_conversations_updated: CallbackFunction,
    // pub(super) cb_on_conversation_removed: CallbackFunction,
}

#[flutter_rust_bridge::frb(ignore)]
impl restsend_sdk::callback::Callback for CallbackDartWrap {
    fn on_connected(&self) {
        if let Some(cb) = self.cb_on_connected.lock().unwrap().as_ref() {
            cb();
        }
    }
    
    fn on_connecting(&self){}
    
    fn on_token_expired(&self,reason:String){}
    
    fn on_net_broken(&self,reason:String){}
    
    fn on_kickoff_by_other_client(&self,reason:String){}
    
    fn on_system_request(&self,req:ChatRequest) -> Option<ChatRequest>{
    None
    }
    
    fn on_unknown_request(&self,req:ChatRequest) -> Option<ChatRequest>{
    None
    }
    
    fn on_topic_typing(&self,topic_id:String,message:Option<String>){}
    
    fn on_new_message(&self,topic_id:String,message:ChatRequest) -> bool {
    return false;
    }
    
    fn on_topic_read(&self,topic_id:String,message:ChatRequest){}
    
    fn on_conversations_updated(&self,conversations:Vec<Conversation>){}
    
    fn on_conversation_removed(&self,conversation_id:String){}
}


impl super::client::Client {
    pub fn set_onconnected(&self, callback:impl Fn() + Send + Sync ) {
        //*self.cb_on_connected.lock().unwrap() = callback;
    }    
}