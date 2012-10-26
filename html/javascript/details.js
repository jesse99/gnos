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
	
	var queries = ['					\
SELECT 								\
	?detail								\
WHERE 								\
{										\
	?subject gnos:target {0} . 			\
	?subject gnos:detail ?detail . 		\
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

function details_query(solution)
{
	var html = '';
	$.each(solution, function (i, row)
	{
		var data = JSON.parse(row.detail);
		html += detail_to_html(data);
	});
	
	if (!html)
		html = "<p>No details</p>";
	
	html = "<h2>{0}</h2>\n{1}".format(escapeHtml(GNOS.label), html);
	
	return {details: html};
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

function detail_to_html(data)
{
	var html = '';
	
	if ('markdown' in data)
	{
		html = "<p>" + markdown.toHTML(data.markdown) + "</p>";
	}
	else if ('accordion' in data)
	{
		html = accordion_to_html(data.accordion);
	}
	else if ('table' in data)
	{
		html = table_to_html(data.table);
	}
	else
	{
		console.log("bad detail: {0:j}".format(data));
	}
	
	return html;
}

// required: title (may be empty), open, key
// optional: markdown, detail
function accordion_to_html(accordion)
{
	var html = '';
	
	$.each(accordion, function (i, data)
	{
		assert('markdown' in data || 'detail' in data, "expected markdown or detail in {0:j}".format(data));
		
		if ('markdown' in data)
			var inner = markdown.toHTML(data.markdown);
		else
			var inner = detail_to_html(data.detail);
		
		if (data.open === "always")
		{
			html += inner;
		}
		else
		{
			var open = GNOS.opened[data.key] || data.open === "yes";
			GNOS.opened[data.key] = open;
			
			var handler = "GNOS.opened['{0}'] = !GNOS.opened['{0}']".format(data.key);
			if (open)
				html += '<details open="open" onclick = "{0}">\n'.format(handler);
			else
				html += '<details onclick = "{0}">\n'.format(handler);
				
			if (data.title)
				html += '<summary>{0}</summary>\n'.format(escapeHtml(data.title));
				
			html += '{0}\n'.format(inner);
			html += '</details>\n';
		}
	});
	
	return html;
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
	var html = '';
	
	if (model.alerts)
	{
		html += model.alerts + '\n';
	}
	
	html += model.details;
	
	$('#body').html(html);
}
