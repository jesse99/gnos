"use strict";

function bolder(style)
{
	var result = clone(style);
	result.fontWeight += 300;
	if (result.fontWeight > 900)
		result.fontWeight = 900;
	return result;
}

function xlarger(style)
{
	var result = clone(style);
	result.fontSize = Math.round(1.4*result.fontSize);
	return result;
}

function larger(style)
{
	var result = clone(style);
	result.fontSize = Math.round(1.2*result.fontSize);
	return result;
}

function smaller(style)
{
	var result = clone(style);
	result.fontSize = Math.round(0.8*result.fontSize);
	return result;
}

var styles =
{
	'default':
	{
		fontStyle: 'normal',		// or italic, oblique
		fontWeight: 400,		// 100 to 900 where normal is 400 and 700 is bold
		fontSize: 12,				// in points
		fontFamily: 'arial',		// font name (TODO: add web safe fonts) or serif, sans-serif, cursive, monospace
	},
	'default_object':   {},
	'primary_label':    {fontWeight: bolder, fontSize: xlarger},
	'secondary_label': {},
	'tertiary_label':    {fontSize: smaller},
};

function compose_styles(names)
{
	var style = styles['default'];
	
	for (var i=0; i < names.length; ++i)
	{
		var name = names[i];
		if (name in styles)
		{
			var rhs = styles[name];
			for (var key in rhs)
			{
				if (rhs[key] instanceof Function)
					style = rhs[key](style);
				else
					style[key] = rhs[key];
			}
		}
		else
		{
			console.log("'{0}' is not a known style".format(name));
		}
	}
	
	return style;
}

function apply_styles(context, names)
{
	var style = compose_styles(names);
	
	var font = '';
	font += style.fontStyle + " ";
	font += style.fontWeight + " ";
	font += style.fontSize + "pt ";
	font += style.fontFamily + " ";
	
	context.font = font;
}

