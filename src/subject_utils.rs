import std::map::hashmap;
import rrdf::*;

export str_map, object_to_context, str_map_to_str, str_maps_to_str;

type str_map = std::map::hashmap<str, str>;

fn append_uri(&urls: [str_map], url: str, name: str)
{
	vec::push(urls, std::map::hash_from_strs([("url", #fmt["/subject/%s", url]), ("text", name)]));
}

fn append_normal(&normals: [str_map], value: str)
{
	vec::push(normals, std::map::hash_from_strs([("object", value)]));
}

fn append_reference(&urls: [str_map], subject: subject)
{
	alt subject
	{
		iri(v)		{append_uri(urls, v, v)}
		blank(v)	{append_uri(urls, v, v)}
	};
}

fn append_primitive(&normals: [str_map], &urls: [str_map], primitive: primitive)
{
	alt primitive
	{
		anyURI(v)		{append_uri(urls, v, v)}
		_				{append_normal(normals, primitive.to_str())}
	};
}

fn append_scalar(&normals: [str_map], &urls: [str_map], scalar: object)
{
	alt scalar
	{
		reference(v)			{append_reference(urls, v)}
		primitive(v)			{append_primitive(normals, urls, v)}
		typed_literal(v, _)	{append_normal(normals, v)}
		plain_literal(v, _)	{append_normal(normals, v)}
		_						{fail}	// TODO: could probably support this by flattening the nested lists into one list
	};
}

fn append_list(&normals: [str_map], &urls: [str_map], list: [object])
{
	for vec::each(list)
	{|e|
		append_scalar(normals, urls, e);
	}
}

// Returns maps looking like:
//    (
// 	   "url-objects" : [{"url": value, "text": value}],
// 	   "normal-objects" : [{"object": value}]
//    )
// If the object is a scalar one list will have a single value. If it is a
// seq, bag, or alt_ each list may have multiple values (which must be scalars).
fn object_to_context(object: object) -> ([str_map], [str_map])
{
	let mut url_objects = [];
	let mut normal_objects = [];
	
	alt object
	{
		reference(v)			{append_reference(url_objects, v)}
		primitive(v)			{append_primitive(normal_objects, url_objects, v)}
		typed_literal(v, _)	{append_normal(normal_objects, v)}
		plain_literal(v, _)	{append_normal(normal_objects, v)}
		seq(v)					{append_list(normal_objects, url_objects, v)}
		bag(v)					{append_list(normal_objects, url_objects, v)}
		alt_(v)					{append_list(normal_objects, url_objects, v)}
	}
	
	(url_objects, normal_objects)
}

fn map_to_vector<K: copy, V: copy>(map: std::map::hashmap<K, V>) -> [(K, V)]
{
	let mut result = [];
	vec::reserve(result, map.size());
	
	for map.each()
	{|key, value|
		vec::push(result, (key, value));
	};
	result
}

// TODO: would be better to have a to_str impl, but was getting multiple definition errors...
fn str_map_to_str(self: str_map) -> str
{
	let v = map_to_vector(self);
	"{" + str::connect(vec::map(v, {|t| #fmt["\"%s\" => \"%s\"", tuple::first(t), tuple::second(t)]}), ", ") + "}"
}

fn str_maps_to_str(self: [str_map]) -> str
{
	"[" + str::connect(vec::map(self, {|f| str_map_to_str(f)}), ", ") + "]"
}
