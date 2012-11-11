// Cascading style sheets sort of thing applied to shapes.
"use strict";

// Table mapping style names to callbacks applying the style.
GNOS.handlers =
{
	'font-size': font_size,
	'font-family': font_family,
	'font-style': font_style,
	'font-weight': font_weight,
	'font-color': colors,
	
	'frame-width': frame_width,
	'frame-color': stroke_color,
	'frame-back-color': fill_color,
	'frame-blur': frame_blur,
	
	'line-width': frame_width,
	'line-color': stroke_color,
	'line-type': no_op,					// this is handled by LineShape
	'line-arrow-width': arrow_width,
	'line-arrow-height': arrow_height,
	
	'gauge-bar-color': fill_color
};

// Applies cascading styles to the current canvas context.
function apply_styles(context, styles)
{
//console.log("styles: {0:j}".format(styles));
	context.font_parts = ['normal', 400, 12, 'Arial'];	// font-style, font-weight, font-size, font-family
	context.lineWidth = 1;
	context.strokeStyle = 'black';
	context.fillStyle = 'black';
	context.frameBlur = undefined;
	context.arrow_width = undefined;
	context.arrow_height = undefined;
	
	$.each(styles, function (k, style)
	{
		if (style)
		{
			var i = style.indexOf(':');
			assert(i > 0, "failed to find ':' in " + style);
			
			var name = style.slice(0, i);
			var value = style.slice(i+1);
			
			var handler = GNOS.handlers[name];
			assert(handler, "failed to find a style handler for " + name);
			handler(context, value);
		}
	});
	
	if (context.arrow_width === undefined)
		context.arrow_width = 12 + context.lineWidth;
	if (context.arrow_height === undefined)
		context.arrow_height = 15 + context.lineWidth;
	
	context.font_parts[1] = context.font_parts[1].toFixed();
	context.font_parts[2] = context.font_parts[2].toFixed() + 'pt';
	context.font = context.font_parts.join(' ');
//console.log("   fill: {0}".format(context.fillStyle));
}

function font_style(context, value)
{
	context.font_parts[0] = value;
}

function font_weight(context, value)
{
	if (value === 'bolder')
	{
		var weight = context.font_parts[1] + 300;
		if (weight > 900)
			weight = 900;
		context.font_parts[1] = weight;
	}
	else if (value == 'lighter')
	{
		var weight = context.font_parts[1] - 300;
		if (weight < 100)
			weight = 100;
		context.font_parts[1] = weight;
	}
	else
	{
		context.font_parts[1] = parseInt(value, 10);
	}
}

function font_size(context, value)
{
	if (value === 'xxx-small')
	{
		context.font_parts[2] = 6;
	}
	else if (value === 'xx-small')
	{
		context.font_parts[2] = 8;
	}
	else if (value === 'x-small')
	{
		context.font_parts[2] = 9;
	}
	else if (value === 'small')
	{
		context.font_parts[2] = 10;
	}
	else if (value === 'normal')
	{
		context.font_parts[2] = 12;
	}
	else if (value === 'medium')
	{
		context.font_parts[2] = 12;
	}
	else if (value === 'large')
	{
		context.font_parts[2] = 16;
	}
	else if (value === 'x-large')
	{
		context.font_parts[2] = 20;
	}
	else if (value === 'xx-large')
	{
		context.font_parts[2] = 24;
	}
	else if (value === 'larger')
	{
		var size = Math.round(1.2*context.font_parts[2]);
		context.font_parts[2] = size;
	}
	else if (value == 'smaller')
	{
		var size = Math.max(Math.round(0.8*context.font_parts[2]), 8);
		context.font_parts[2] = size;
	}
	else
	{
		assert(false, "bad font-size: " + value);
	}
}

function font_family(context, value)
{
	context.font_parts[3] = value;
}

function frame_width(context, value)
{
	var x = parseFloat(value);
	if (x === 0.0)
		context.lineWidth = 0.00001;		// setting lineWidth to 0.0 actually sets it to 1.0
	else
		context.lineWidth = x;
}

function color_to_css(value)
{
	var i = value.indexOf('/');
	if (i > 0)
	{
		var color = value.slice(0, i);
		var alpha = parseFloat(value.slice(i+1));
		
		var triplet = Color.get(color).hexTriplet();
		return new Color(triplet, alpha).css();
	}
	else
	{
		return Color.get(value).css();
	}
}

function colors(context, value)
{
	var color = color_to_css(value);
	
	context.strokeStyle = color;
	context.fillStyle = color;
}

function stroke_color(context, value)
{
	var color = color_to_css(value);
	
	context.strokeStyle = color;
}

function fill_color(context, value)
{
	var color = color_to_css(value);
	
	context.fillStyle = color;
}

function frame_blur(context, value)
{
	context.frameBlur = parseInt(value, 10);
}

function arrow_width(context, value)
{
	context.arrow_width = parseInt(value, 10);
}

function arrow_height(context, value)
{
	context.arrow_height = parseInt(value, 10);
}

function no_op(context, value)
{
}

// This is a very common way to adjust the lightness of a color, but it's not a very
// good way because linear changes in HSL lightness are not perceived as linear changes
// by the eye. Something like CIELAB would probably work better.
//
// But, for now, we're hard-coding these adjustments so it doesn't really matter.
function scale_lightness(color_name, scaling)
{
	var color = Color.get(color_name);
	var hsl = color.hslData();
	hsl[2] = Math.min(scaling*hsl[2], 1.0);
	return Color.hsl(hsl[0], hsl[1], hsl[2]).css();
}
