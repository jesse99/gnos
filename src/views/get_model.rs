// Provides a view onto the back-end model used by gnos. This is not intended
// to be something used very often: it's just a convenient mechanism by which
// users can inspect the raw data used by the other views..
import rrdf::object::object_methods;
import rrdf::solution::solution_row_methods;

fn get_subjects(state_chan: comm::chan<msg>, _request: server::request, response: server::response) -> server::response
{
	let rows = get_state(state_chan, "
		PREFIX gnos: <http://www.gnos.org/2012/schema#>
		SELECT DISTINCT
			?name
		WHERE
		{
			?subject ?predicate ?object .
			BIND(rrdf:pname(?subject) AS ?name)
		} ORDER BY ?name");

	let mut subjects = [];
	
	for vec::eachi(rows)
	{|index, row|
		let name = row.get("name").as_str();
		
		let map = std::map::str_hash();
		map.insert("row-class", mustache::str(if index & 1u == 0u {"even"} else {"odd"}));
		map.insert("url", mustache::str(#fmt["/subject/%s", name]));
		map.insert("text", mustache::str(name));
		vec::push(subjects, mustache::map(map));
	}
	
	response.context.insert("subjects", mustache::vec(subjects));
	{template: "(private)/subjects.html" with response}
}

fn get_subject(_state_chan: comm::chan<msg>, _request: server::request, response: server::response) -> server::response
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

