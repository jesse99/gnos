"use strict";

// from and to should have unit scaled x and y properties.
function draw_line(context, styles, from, to)
{
	context.save();
	apply_styles(context, styles);
	context.beginPath();
	
	var x = from.x * context.canvas.width;
	var y = from.y * context.canvas.height;
	context.moveTo(x, y);
	
	x = to.x * context.canvas.width;
	y = to.y * context.canvas.height;
	context.lineTo(x, y);
	
	context.stroke();
	context.restore();
}

// Draw lines of text centered on (x, y). This is a bit complex because
// each line may be styled differently.
function center_text(context, base_styles, lines, styles, x, y)
{
	if (lines)
	{
		context.save();
		
		context.textAlign = 'center';
		context.textBaseline = 'top';
		context.fillStyle = 'black';
		
		var heights = compute_line_heights(context, base_styles, styles);
		var total_height = heights.reduce(function(previous, current, i, array)
		{
			return previous + current;
		}, 0);
		y -= total_height/2;
		
		for (var i=0; i < lines.length; ++i)
		{
			var line = lines[i];
			apply_styles(context, base_styles.concat(styles[i]));
			
			context.fillText(line, x, y);
			y += heights[i];
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

