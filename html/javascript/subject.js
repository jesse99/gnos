"use strict";

function make_link(url, label)
{
	if (url != label)
	{
		// url: http://www.gnos.org/2012/schema#foo
		// label: gnos:foo
		return escapeHtml(label);
	}
	else
	{
		if (url.indexOf("http://") == 0)
		{
			// url & value: http://some/random/web/site#foo
			// Shouldn't normally hit this case.
			return '<a href="{0}">{1}</a>'.format(encodeURI(url), escapeHtml(url.substr(0, url.length - 7)));
		}
		else if (url.indexOf("_:") == 0)
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

window.onload = function()
{
	var table = document.getElementById('subject');
	var expr = '																\
PREFIX devices: <http://network/>										\
PREFIX gnos: <http://www.gnos.org/2012/schema#>					\
SELECT 																		\
	?predicate_url ?predicate_label ?value_url ?value_label			\
WHERE 																		\
{																				\
	{0} ?predicate_url ?value . 											\
	BIND(rrdf:pname(?predicate_url) AS ?predicate_label) .			\
	BIND(isIRI(?value) || isBlank(?value) AS ?is_url) .					\
	BIND(IF(?is_url, ?value, "") AS ?value_url) .						\
	BIND(IF(?is_url, rrdf:pname(?value), ?value) AS ?value_label)	\
} ORDER BY ?predicate_label ?value_label'.format(table.getAttribute("data-about"));

	var source = new EventSource('/query?name=model&expr='+encodeURIComponent(expr));
	source.addEventListener('message', function(event)
	{
		var data = JSON.parse(event.data);
		
		var html = '';
		for (var i=0; i < data.length; ++i)
		{
			var row = data[i];
			//console.log('predicate_url: "{0}", predicate_label: "{1}", value_url: "{2}", value_label: "{3}"'.format(row.predicate_url, row.predicate_label, row.value_url, row.value_label));
			var klass = i & 1 ? "odd" : "even";
			html += '<tr class="{0}">'.format(klass);
			
			html += '	<td class="predicate">';
			html += '	{0}'.format(make_link(row.predicate_url, row.predicate_label));
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
		table.innerHTML = html;
	});
	
	source.addEventListener('open', function(event)
	{
		console.log('> subject stream opened');
	});
	
	source.addEventListener('error', function(event)
	{
		if (event.eventPhase == 2)
		{
			console.log('> subject stream closed');
		}
	});
}
