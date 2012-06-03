// Provides a view onto the back-end model used by gnos. This is not intended
// to be something used very often: it's just a convenient mechanism by which
// users can inspect the raw data used by the other views..

// TODO: This has bit-rotted a bit.
fn get_model(_options: options, _settings: hashmap<str, str>, _request: server::request, response: server::response) -> server::response
{
	//let subject = request.matches.get("subject");
	//let matches = vec::filter(graph) {|elem| elem.subject == iri(subject)};
	
	//fn le(&&a: triple, &&b: triple) -> bool {a.property <= b.property}
	//let matches = std::sort::merge_sort(le, matches);
	
	//let mut properties = [];
	//for vec::eachi(matches)
	//{|index, match|
	//	let map = std::map::str_hash();
	//	let (urls, normals) = subject_utils::object_to_context(match.object);
	//	map.insert("row-class", mustache::str(if index & 1u == 0u {"even"} else {"odd"}));
	//	map.insert("property", mustache::str(match.property));
	//	map.insert("has-urls", vec::is_not_empty(urls).to_mustache());
	//	map.insert("url-objects", urls.to_mustache());
	//	map.insert("normal-objects", normals.to_mustache());
	//	vec::push(properties, mustache::map(map));
	//};
	
	//response.context.insert("subject", mustache::str(subject));
	//response.context.insert("properties", mustache::vec(properties));
	
	{template: "(private)/subject.html" with response}
}

