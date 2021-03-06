/// Displays the subjects used by the various gnos stores. This is not intended
/// to be something used very often: it's just a convenient mechanism by which
/// users can inspect the raw data used by the other views.
use  mustache::*;
use server = rwebserve;
use Config = rwebserve::Config;
use Request = rwebserve::Request;
use Response = rwebserve::Response;
use ResponseHandler = rwebserve::ResponseHandler;

pub fn get_models(options: &options::Options, response: server::Response, _state_chan: oldcomm::Chan<model::Msg>) -> server::Response
{
	response.context.insert(@~"network-name", Str(@copy options.network_name));
	
	// There isn't much for us to do here: all the heavy lifting is done on
	// the client via javascript that uses server-sent events to dynamically
	// update the view based on a SPARQL query.
	server::Response {template: ~"models.html", ..response}
}
