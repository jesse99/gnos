// Form that allows users to run an arbitrary SPARQL query against a store. 
use  mustache::*;
use server = rwebserve::rwebserve;
use ConnConfig = rwebserve::connection::ConnConfig;
use Request = rwebserve::rwebserve::Request;
use Response = rwebserve::rwebserve::Response;
use ResponseHandler = rwebserve::rwebserve::ResponseHandler;

fn get_query_store(options: &options::Options, _request: &server::Request, response: &server::Response) -> server::Response
{
	response.context.insert(@~"network-name", mustache::Str(@copy options.network_name));
	
	server::Response {template: ~"query-store.html", ..*response}
}
