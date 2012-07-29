"use strict";

// All coordinates are screen coordinates.
function Point(x, y)
{
	this.x = x;
	this.y = y;
}

// from and to are Points.
function draw_line(context, styles, from, to)
{
	context.save();
	apply_styles(context, styles);
	context.beginPath();
	
	context.moveTo(from.x, from.y);
	context.lineTo(to.x, to.y);
	
	context.stroke();
	context.restore();
}

// Draws a filled disc. If lineWidth is non-zero a border is also added.
// at is a Point.
function draw_disc(context, styles, at, radius)
{
	context.save();
	apply_styles(context, styles);
	//console.log("drawing disc with {0:j} and {1:j}".format(styles, compose_styles(styles)));
	
	context.beginPath();
	context.arc(at.x, at.y, radius, 0, 2*Math.PI);
	context.closePath();
	
	context.fill();
	if (context.lineWidth != 0)
		context.stroke();
	context.restore();
}

// Returns an object with:
//    total_height: of all the lines
//    max_width: width of the widest line
// and misc fields for internal use.
function prep_center_text(context, base_styles, lines, styles)
{
	var total_height = 0.0;
	var max_width = 0.0;
	var heights = [];
	
	if (lines)
	{
		context.save();
		
		context.textAlign = 'center';
		context.textBaseline = 'top';
		
		heights = compute_line_heights(context, base_styles, styles);
		total_height = heights.reduce(function(previous, current, i, array)
		{
			return previous + current;
		}, 0);
		
		for (var i=0; i < lines.length; ++i)
		{
			var line = lines[i];
			apply_styles(context, base_styles.concat(styles[i]));
			
			var metrics = context.measureText(line);
			max_width = Math.max(metrics.width, max_width);
		}
		
		context.restore();
	}
	
	return {total_height: total_height, max_width: max_width, heights: heights};
}

// Draw lines of text centered on a Point. This is a bit complex because
// each line may be styled differently.
function center_text(context, base_styles, lines, styles, center, stats)
{
	if (lines)
	{
		context.save();
		
		context.textAlign = 'center';
		context.textBaseline = 'top';
		context.fillStyle = 'black';
		
		var y = center.y - stats.total_height/2;
		
		for (var i=0; i < lines.length; ++i)
		{
			var line = lines[i];
			apply_styles(context, base_styles.concat(styles[i]));
			
			context.fillText(line, center.x, y);
			y += stats.heights[i];
		}
		
		context.restore();
	}
}

function compute_line_heights(context, base_styles, styles)
{
	var heights = [];
	context.save();
	
	for (var i=0; i < styles.length; ++i)
	{
		apply_styles(context, base_styles.concat(styles[i]));
		
		var line_height = compute_line_height(context);
		heights.push(line_height);
	}
	
	context.restore();
	return heights;
}

function compute_line_height(context)
{
	var metrics = context.measureText("W");
	
	// As of July 2012 Chrome only has width inside metrics.
	if ('fontBoundingBoxAscent' in metrics)
		var line_height = metrics.fontBoundingBoxAscent + metrics.fontBoundingBoxDescent;
	else
		var line_height = metrics.width + metrics.width/6;	// this is what Core HTML5 Canvas recommends
		
	return line_height;
}

