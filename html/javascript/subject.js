// Page that shows predicates for a particular subject.
"use strict";

$(document).ready(function()
{
	var table = document.getElementById('subject');
	var store = table.getAttribute("data-name");
	var about = table.getAttribute("data-about");
	
	var query = '														\
SELECT 																\
	?predicate_url ?predicate_label ?value_url ?value_label			\
WHERE 																\
{																		\
	{0} ?predicate_url ?value . 										\
	BIND(rrdf:pname(?predicate_url) AS ?predicate_label) .			\
	BIND(isIRI(?value) || isBlank(?value) AS ?is_url) .				\
	BIND(IF(?is_url, ?value, "") AS ?value_url) .					\
	BIND(IF(?is_url, rrdf:pname(?value), ?value) AS ?value_label)	\
} ORDER BY ?predicate_label ?value_label'.format(about);

	register_query("subject", ["subject"], store, [query], [subject_query]);
	
	register_renderer("subject", ["subject"], "subject", subject_renderer);
});

function subject_query(solution)
{
	var html = '';
	for (var i = 0; i < solution.length; ++i)
	{
		var row = solution[i];
		//console.log('predicate_url: "{0}", predicate_label: "{1}", value_url: "{2}", 
		//	value_label: "{3}"'.format(row.predicate_url, row.predicate_label, row.value_url, row.value_label));
		var klass = i & 1 ? "odd" : "even";
		html += '<tr class="{0}">'.format(klass);
		
		html += '	<td class="predicate">';
		if (row.predicate_label.indexOf("sname:") === 0)
		{
			var name = row.predicate_label.slice("sname:".length);
			var url = "http://tools.cisco.com/Support/SNMP/do/BrowseOID.do?objectInput={0}&translate=Translate&submitValue=SUBMIT&submitClicked=true".format(name);
			html += '<a href="{0}">{1}</a>'.format(encodeURI(url), escapeHtml(name));
		}
		else
		{
			html += '	{0}'.format(make_link(row.predicate_url, row.predicate_label));
		}
		html += '	</td>';
		
		html += '	<td class="value"><span>	';
		if (row.value_url)
		{
			html += '		{0}'.format(make_link(row.value_url, row.value_label));
		}
		else
		{
			html += '		{0}'.format(escapeHtml(row.value_label));
		}
		html += '	</span></td>';
		
		html += '</tr>';
	}
	
	return {subject: html};
}

function subject_renderer(element, model, model_names)
{
	element.innerHTML = model.subject;
}

function make_link(url, label)
{
	if (url !== label)
	{
		// url: http://www.gnos.org/2012/schema#foo
		// label: gnos:foo
		return escapeHtml(label);
	}
	else
	{
		if (url.indexOf("http://") === 0)
		{
			// url & value: http://some/random/web/site#foo
			// Shouldn't normally hit this case.
			return '<a href="{0}">{1}</a>'.format(encodeURI(url), escapeHtml(url.substr(0, url.length - 7)));
		}
		else if (url.indexOf("_:") === 0)
		{
			// url & value: _:blank-node
			return '<a href="/subject/{0}">{1}</a>'.format(encodeURIComponent(url), escapeHtml(url));
		}
		else
		{
			// url & value: something that isn't http
			// Shouldn't hit this case.
			return escapeHtml(label);
		}
	}
}

