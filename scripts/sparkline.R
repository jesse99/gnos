# Creates a sparkline for a sample set.
# These are small graphics that can be embedded inline with text content.
# We use them to show aggregate network bandwidth as well as bandwidths
# for individual interfaces.
#
# When the server has to generate multiple sparklines it will concatenate
# together multiple copies of this file to avoid starting up Rscript multiple
# times (which is why the YaleToolkit load is commented out).
#library(YaleToolkit)

samples = c({{samples}});

png("{{file}}", {{width}}, {{height}})

grid.newpage()
pushViewport(viewport(w = 0.85, h = 0.85))	# http://rgm2.lab.nig.ac.jp/RGM2/func.php?rd_id=grid:viewport

# http://rgm2.lab.nig.ac.jp/RGM2/func.php?rd_id=YaleToolkit:sparkline
# Note that labels can be added via the grid.text function (or main and sub arguments here).
sparkline(
	samples,
	ptopts = list(labels = 'min.max'),			# add labels for the min and max samples
	IQR = gpar(fill = 'cornsilk', col = 'cornsilk'),	# show inter-quartile range
	buffer = unit(4, 'points'),
	new = FALSE,
	ylim = c(0.0, max(samples))
)

popViewport()
dev.off()
########################################################

