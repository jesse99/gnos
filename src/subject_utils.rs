import std::map::hashmap;
import rrdf::object::*;
import rrdf::store::*;
import mustache::to_mustache;

export str_map, object_to_context;

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

// Returns maps looking like:
//    (
// 	   "url-objects" : [{"url": value, "text": value}],
// 	   "normal-objects" : [{"object": value}]
//    )
// If the object is a scalar one list will have a single value. If it is a
// seq, bag, or alt_ each list may have multiple values (which must be scalars).
fn object_to_context(object: object, context: str_map)
{
	// TODO: scalar discussion above is out of date: do we want to special case
	// rdf containers?
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

fn map_to_vector<K: copy, V: copy>(map: std::map::hashmap<K, V>) -> ~[(K, V)]
{
	let mut result = []/~;
	vec::reserve(result, map.size());
	
	for map.each()
	|key, value|
	{
		vec::push(result, (key, value));
	};
	result
}

// TODO: would be better to have a to_str impl, but was getting multiple definition errors...
//fn str_map_to_str(self: str_map) -> str
//{
//	let v = map_to_vector(self);
//	"{" + str::connect(vec::map(v, |t| {#fmt["\"%s\" => \"%s\"", tuple::first(t), tuple::second(t)]}), ", ") + "}"
//}
//
//fn str_maps_to_str(self: [str_map]) -> str
//{
//	"[" + str::connect(vec::map(self, |f| {str_map_to_str(f)}), ", ") + "]"
//}
