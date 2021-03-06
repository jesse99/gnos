/// Form that allows users to run an arbitrary SPARQL query against a store. 
use  mustache::*;
use server = rwebserve;
use Config = rwebserve::Config;
use Request = rwebserve::Request;
use Response = rwebserve::Response;
use ResponseHandler = rwebserve::ResponseHandler;

pub fn get_query_store(options: &options::Options, _request: &server::Request, response: server::Response) -> server::Response
{
	response.context.insert(@~"network-name", Str(@copy options.network_name));
	
	server::Response {template: ~"query-store.html", ..response}
}
