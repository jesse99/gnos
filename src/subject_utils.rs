import std::map::hashmap;
import rrdf::object::*;
import rrdf::store::*;

export str_map, object_to_context, str_map_to_str, str_maps_to_str;

type str_map = std::map::hashmap<str, str>;

fn append_uri(&urls: [str_map]/~, url: str, name: str)
{
	vec::push(urls, std::map::hash_from_strs([("url", #fmt["/subject/%s", url]), ("text", name)]));
}

fn append_normal(&normals: [str_map]/~, value: str)
{
	vec::push(normals, std::map::hash_from_strs([("object", value)]));
}

// Returns maps looking like:
//    (
// 	   "url-objects" : [{"url": value, "text": value}],
// 	   "normal-objects" : [{"object": value}]
//    )
// If the object is a scalar one list will have a single value. If it is a
// seq, bag, or alt_ each list may have multiple values (which must be scalars).
fn object_to_context(object: object) -> ([str_map]/~, [str_map]/~)
{
	let mut url_objects = []/~;
	let mut normal_objects = []/~;
	
	// TODO: scalar discussion above is out of date: do we want to special case
	// rdf containers?
	alt object
	{
		iri_value(_)			{append_uri(normal_objects, object.to_str(), object.to_str())}
		blank_value(_)		{append_uri(normal_objects, object.to_str(), object.to_str())}
		
		// TODO: need to special case these and use some error css
		unbound_value(_)	{append_normal(normal_objects, object.to_str())}
		invalid_value(*)		{append_normal(normal_objects, object.to_str())}
		error_value(_)		{append_normal(normal_objects, object.to_str())}
		
		_						{append_normal(normal_objects, object.to_str())}
	}
	
	(url_objects, normal_objects)
}

fn map_to_vector<K: copy, V: copy>(map: std::map::hashmap<K, V>) -> [(K, V)]/~
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
fn str_map_to_str(self: str_map) -> str
{
	let v = map_to_vector(self);
	"{" + str::connect(vec::map(v, |t| {#fmt["\"%s\" => \"%s\"", tuple::first(t), tuple::second(t)]}), ", ") + "}"
}

fn str_maps_to_str(self: [str_map]) -> str
{
	"[" + str::connect(vec::map(self, |f| {str_map_to_str(f)}), ", ") + "]"
}
