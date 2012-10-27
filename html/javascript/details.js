// Page that shows details for a particular subject.
"use strict";

GNOS.store = undefined;
GNOS.opened = {};

$(document).ready(function()
{
	var table = $('#body');
	GNOS.store = table.attr("data-name");
	GNOS.label = table.attr("data-label");
	var about = table.attr("data-about");
	
	var target = about.replace('/', ':');
	var oldest = new Date();
	oldest.setDate(oldest.getDate() - 7);	// show alerts for the last week
	
	var queries = ['						\
SELECT 									\
	?detail ?title ?open ?sort_key ?key	\
WHERE 									\
{											\
	?subject gnos:target {0} . 				\
	?subject gnos:detail ?detail . 			\
	?subject gnos:title ?title .	 			\
	?subject gnos:open ?open .	 		\
	?subject gnos:sort_key ?sort_key .	 \
	?subject gnos:key ?key .	 			\
}'.format(target),
'SELECT 											\
	?mesg ?resolution ?style ?begin ?end			\
WHERE 											\
{													\
	?subject gnos:target {0} .						\
	?subject gnos:begin ?begin .					\
	?subject gnos:mesg ?mesg .					\
	?subject gnos:resolution ?resolution .			\
	?subject gnos:style ?style .						\
	OPTIONAL									\
	{												\
		?subject gnos:end ?end					\
	}												\
	FILTER (?begin >= "{1}"^^xsd:dateTime)	\
} ORDER BY ?begin ?mesg'.format(target, oldest.toISOString())
];

	var model_names = ['details', 'alerts'];
	register_query("details", model_names, GNOS.store, queries, [details_query, alerts_query]);
	
	register_renderer("details", model_names, "body", details_renderer);
});

// solution rows have 
// required fields: detail, title, open, sort_key, key
function details_query(solution)
{
	var items = [];
	$.each(solution, function (i, row)
	{
		var inner = detail_to_html(row.detail);
		
		if (row.open === "always")
		{
			var html = inner;
		}
		else
		{
			var open = GNOS.opened[row.key] || row.open === "yes";
			GNOS.opened[row.key] = open;
			
			var handler = "GNOS.opened['{0}'] = !GNOS.opened['{0}']".format(row.key);
			if (open)
				var html = '<details open="open" onclick = "{0}">\n'.format(handler);
			else
				var html = '<details onclick = "{0}">\n'.format(handler);
				
			if (row.title)
				html += '<summary>{0}</summary>\n'.format(escapeHtml(row.title));
				
			html += '{0}\n'.format(inner);
			html += '</details>\n';
		}
		
		items.push({sort_key: row.sort_key, html: html});
	});
	
	items.sort(function (x, y)
	{
		if (x.sort_key < y.sort_key)
			return -1;
		else if (x.sort_key > y.sort_key)
			return 1;
		else
			return 0;
	});
	
	var items = items.map(function (x) {return x.html;});
	var html = items.join('\n');
	
	if (!html)
		html = "<p>No details</p>";
	
	return {details: html};
}

function detail_to_html(detail)
{
	var html = '';
	
	if (detail && detail[0] === '{')
	{
		try
		{
			var data = JSON.parse(detail);
			html = table_to_html(data);
		}
		catch (e)
		{
			// rare case where markdown starts with {
			html = markdown.toHTML(detail);
		}
	}
	else
	{
		html = markdown.toHTML(detail);
	}
	
	return html;
}

// solution rows have 
// required fields: mesg, resolution, style, begin
// optional fields: end, target
function alerts_query(solution)
{
	function add_alert(row, options)
	{
		var html = "";
		if (options.styles.indexOf(row.style) >= 0 && (options.kind === "inactive") === 'end' in row)
		{
			if ('end' in row)
				var date = new Date(row.end);
			else
				var date = new Date(row.begin);
				
			if ('target' in row)
			{
				var i = row.target.lastIndexOf('#');
				if (i < 0)
					i = row.target.lastIndexOf('/');
					
				if (i >= 0)
					var target = "{0}: ".format(row.target.slice(i+1));
				else
					var target = "{0}: ".format(row.target);
			}
			else
				var target = "";
				
			var lines = row.mesg.split("\n");
			for (var i = 0; i < lines.length; ++i)
			{
				var attributes = '';
				var classes = row.style.replace(':', '-');
				if (i === 0)
				{
					var targets = escapeHtml(target);
					if (row.resolution)
					{
						classes += ' tooltip';
						attributes += ' data-tooltip=" {0}"'.format(escapeHtml(row.resolution));
					}
					var dates = " ({0})".format(dateToStr(date));
				}
				else
				{
					var targets = "";
					classes += ' indent';
					var dates = "";
				}
				
				html += '<li class="{0}"{1}">{2}{3}{4}</li>\n'.format(
					classes, attributes, targets, escapeHtml(lines[i]), dates);
			}
		}
		return html;
	}
	
	function add_widget(inner, title, open)
	{
		var html = "";
		
		if (inner)
		{
			if (open)
				html += '<details open="open">\n';
			else
				html += '<details>\n';
			html += '	<summary>{0}</summary>\n'.format(title);
			html += "		<ul class='sequence'>\n";
			html += inner;
			html += "		</ul>\n";
			html += '</details>\n';
		}
		
		return html;
	}
	
	var error_alerts = "";
	var warning_alerts = "";
	var info_alerts = "";
	var closed_alerts = "";
	
	$.each(solution, function (i, row)
	{
console.log('row: {0:j}'.format(row));
		error_alerts      += add_alert(row, {styles: ["alert-type:error"], kind: "active"});
		warning_alerts += add_alert(row, {styles: ["alert-type:warning"], kind: "active"});
		info_alerts 		+= add_alert(row, {styles: ["alert-type:info"], kind: "active"});
		closed_alerts    += add_alert(row, {styles: ["alert-type:error", "alert-type:warning"], kind: "inactive"});
	});
	
	var html = "";
	html += add_widget(error_alerts, "Error Alerts", true);
	html += add_widget(warning_alerts, "Warning Alerts", false);
	html += add_widget(info_alerts, "Info Alerts", false);
	html += add_widget(closed_alerts, "Closed Alerts", false);
	
	return {alerts: html};
}

// required: style, header, rows
function table_to_html(table)
{
	var html = '';
	
	html += '<table border="1">\n';
		html += '<tr>\n';
		$.each(table.header, function (i, cell)
		{
			html += '<th>{0}</th>\n'.format(escapeHtml(cell));
		});
		html += '</tr>\n';
		
		if (table.style == 'plain')		// tables can be very large so we assume that all the content is formatted in the same way
		{
			$.each(table.rows, function (i, row)
			{
				html += '<tr>\n';
				$.each(row, function (j, cell)
				{
					html += '<td>{0}</td>\n'.format(escapeHtml(cell));
				});
				html += '</tr>\n';
			});
		}
		else if (table.style == 'html')
		{
			$.each(table.rows, function (i, row)
			{
				html += '<tr>\n';
				$.each(row, function (j, cell)
				{
					html += '<td>{0}</td>\n'.format(cell);
				});
				html += '</tr>\n';
			});
		}
		else if (table.style == 'markdown')
		{
			$.each(table.rows, function (i, row)
			{
				html += '<tr>\n';
				$.each(row, function (j, cell)
				{
					html += '<td>{0}</td>\n'.format(markdown.toHTML(cell));
				});
				html += '</tr>\n';
			});
		}
		else
		{
			console.log("bad style: " + table.style);
		}
	html += '</table>\n';
	
	return html;
}

function details_renderer(element, model, model_names)
{
	var html = "<h2>{0}</h2>\n".format(escapeHtml(GNOS.label));
	
	if (model.alerts)
	{
		html += model.alerts + '\n';
		html += '<br>\n';
	}
	
	html += model.details;
	
	$('#body').html(html);
}
