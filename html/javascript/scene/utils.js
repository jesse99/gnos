// Misc general purpose utility functions for canvas.
"use strict";

// Returns a Line between the perimeter of the two discs.
function discs_to_line(disc1, disc2)
{
	if (!disc1.intersects(disc2))
	{
		return new Line(disc1.intersection(disc2.center), disc2.intersection(disc1.center));
	}
	else
	{
		return new Line(Point.zero, Point.zero);
	}
}

// It's a bit tricky to resize the canvas to fill the window but still leave
// room for other html elements. So what we'll do instead is grow the
// canvas as much as we can while retaining the aspect ratio.
function size_to_window(context)
{
	var canvas = context.canvas;
	
	// Set the dimensions of the canvas bitmap. (This corresponds to
	// the html width and height attributes in the html).
	var ratio = canvas.width/canvas.height;
	canvas.width = canvas.parentNode.clientWidth;
	canvas.height = canvas.width/ratio;
	
	// Set the size of the canvas html element. (We need to explicitly 
	// set this to allow other elements on the page to flow correctly).
	canvas.style.width = canvas.width + "px";
	canvas.style.height = canvas.height + "px";
}

function compute_text_metrics(context, lines, base_styles, styles)
{
	assert(lines.length === styles.length, "lines and styles need to match");
	
	var heights = [];
	var widths = [];
	context.save();
	
	for (var i=0; i < styles.length; ++i)
	{
		apply_styles(context, base_styles.concat(styles[i]));
		
		var metrics = do_compute_line_metrics(context, lines[i]);
		heights.push(metrics.line_height);
		widths.push(metrics.width);
	}
	
	context.restore();
	return {heights: heights, widths: widths};
}

// ---- Internal Functions ----------------------------------------------------
function do_compute_line_metrics(context, line)
{
	var metrics = context.measureText("W");
	
	// As of July 2012 Chrome only has width inside metrics. TODO
	//if ('fontBoundingBoxAscent' in metrics)
	//	var line_height = metrics.fontBoundingBoxAscent + metrics.fontBoundingBoxDescent;
	//else
		var line_height = metrics.width + metrics.width/6;	// this is what Core HTML5 Canvas recommends
		
	return {line_height: line_height, width: metrics.width * line.length};
}

