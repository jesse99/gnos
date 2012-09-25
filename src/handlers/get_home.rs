// This is the entry point into gnos web sites. 
use  mustache::*;
use server = rwebserve::rwebserve;

fn get_home(options: options::Options, response: &server::Response) -> server::Response
{
	response.context.insert(@~"admin", mustache::Bool(options.admin));
	response.context.insert(@~"network-name", mustache::Str(@copy options.network_name));
	server::Response {template: ~"home.html", ..*response}
}
