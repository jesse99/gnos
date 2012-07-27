// This is the entry point into gnos web sites. 

fn get_map(options: options::options, response: server::response) -> server::response
{
	response.context.insert(~"admin", mustache::bool(options.admin));
	{template: ~"map.html" with response}
}
