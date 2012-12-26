/// This is the entry point into gnos web sites. 
use  mustache::*;
use server = rwebserve;
use Config = rwebserve::Config;
use Request = rwebserve::Request;
use Response = rwebserve::Response;
use ResponseHandler = rwebserve::ResponseHandler;

pub fn get_home(options: &options::Options, response: server::Response) -> server::Response
{
	response.context.insert(@~"admin", Bool(options.admin));
	response.context.insert(@~"network-name", Str(@copy options.network_name));
	server::Response {template: ~"home.html", ..response}
}
