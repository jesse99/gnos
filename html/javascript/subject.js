// Page that shows predicates for a particular subject.
"use strict";

GNOS.store = undefined;

$(document).ready(function()
{
	var table = $('#subject');
	GNOS.store = table.attr("data-name");
	var about = table.attr("data-about");
	
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

	register_query("subject", ["subject"], GNOS.store, [query], [subject_query]);
	
	register_renderer("subject", ["subject"], "subject", subject_renderer);
});

function subject_query(solution)
{
	var html = '';
	$.each(solution, function (i, row)
	{
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
	});
	
	return {subject: html};
}

function subject_renderer(element, model, model_names)
{
	element.html(model.subject);
}

function make_link(url, label)
{
	if (label.indexOf("gnos:") === 0 || label.indexOf("xsd:") === 0)
	{
		// url: http://www.gnos.org/2012/schema#foo
		// label: gnos:foo
		return escapeHtml(label);
	}
	else
	{
		var parts = label.split(':');
		if (parts.length == 2)
		{
			// url: http://10.6.210.175:8080/map/primary/entities/wall
			// label: entities:wall
			return '<a href="http://localhost:8080/subject/{0}/{1}">{2}</a>'.format(GNOS.store, encodeURI(label), escapeHtml(label));
		}
		else
		{
			// url & value: http://some/random/web/site#foo
			return '<a href="{0}">{1}</a>'.format(encodeURI(url), escapeHtml(url.substr(0, url.length - 7)));
		}
	}
}

