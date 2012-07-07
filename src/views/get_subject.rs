// Provides a view onto the back-end model used by gnos. This is not intended
// to be something used very often: it's just a convenient mechanism by which
// users can inspect the raw data used by the other views..
import rrdf::*;
import mustache::to_mustache;

export get_subject;

type str_map = std::map::hashmap<str, mustache::data>;

fn append_uri(context: str_map, url: str, name: str)
{
	let mut urls = ~[];
	
	let map = std::map::str_hash();
	map.insert("url", mustache::str(#fmt["/subject/%s", url]));
	map.insert("text", mustache::str(name));
	vec::push(urls, map);
	
	context.insert("urls", urls.to_mustache());
}

fn append_normal(context: str_map, value: str)
{
	context.insert("urls", false.to_mustache());
	context.insert("normal", mustache::str(value));
}

fn object_to_context(object: object, context: str_map)
{
	// TODO: do we want to special case containers?
	alt object
	{
		iri_value(_)			{append_uri(context, object.to_str(), object.to_str())}
		blank_value(_)		{append_uri(context, object.to_str(), object.to_str())}
		
		// TODO: need to special case these and use some error css
		unbound_value(_)	{append_normal(context, object.to_str())}
		invalid_value(*)		{append_normal(context, object.to_str())}
		error_value(_)		{append_normal(context, object.to_str())}
		
		_						{append_normal(context, object.to_str())}
	}
}

fn get_subject(state_chan: comm::chan<msg>, request: server::request, response: server::response) -> server::response
{
	let subject = request.matches.get("subject");
	let rows = get_state(state_chan, #fmt["
		PREFIX gnos: <http://www.gnos.org/2012/schema#>
		SELECT
			?predicate ?object
		WHERE
		{
			%s ?p ?object .
			BIND(rrdf:pname(?p) AS ?predicate)
		} ORDER BY ?predicate ?object", subject]);

	let mut predicates = [];
	
	for vec::eachi(rows)
	|index, row|
	{
		let predicate = row.get("predicate").as_str();
		let object = row.get("object");
		
		let map = std::map::str_hash();
		map.insert("row-class", mustache::str(if index & 1u == 0u {"even"} else {"odd"}));
		map.insert("predicate", mustache::str(predicate));
		object_to_context(object, map);
		vec::push(predicates, mustache::map(map));
	}
	
	response.context.insert("subject", mustache::str(subject));
	response.context.insert("predicates", mustache::vec(predicates));
	
	{template: "subject.html" with response}
}

