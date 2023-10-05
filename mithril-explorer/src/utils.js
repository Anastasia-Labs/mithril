function checkUrl(url) {
  try {
    // Use the url constructor to check if the value is an url
    return Boolean(new URL(url));
  } catch (ex) {
    return false;
  }
}

const toAda = (lovelace) => lovelace / 1000000;

const formatCurrency = (number, maximumFractionDigits = 2) => number.toLocaleString(undefined, {
  maximumFractionDigits: maximumFractionDigits,
});

function formatPartyId(partyId) {
  if ((typeof partyId === 'string' || partyId instanceof String) && partyId.length > 15) {
    return partyId.slice(0, 10) + "…" + partyId.slice(-5)
  } else {
    return partyId;
  }
}

function formatStake(lovelace) {
  // Credits to Jasser Mark Arioste for the original idea:
  // https://reacthustle.com/blog/how-to-convert-number-to-kmb-format-in-javascript
  const thresholds = [
    { suffix: 'B', value: 1e9 },
    { suffix: 'M', value: 1e6 },
    { suffix: 'K', value: 1e3 },
    { suffix: '', value: 1 },
  ];
  const ada = toAda(lovelace);
  // Note: subtracting 0.001 to handle cases like `999,999₳` rounding up to `1,000₳` after string format.
  const threshold = thresholds.find((t) => Math.abs(ada) >= (t.value - 0.001));

  if (threshold) {
    return `${formatCurrency(ada / threshold.value)}${threshold.suffix}₳`;
  }

  return `${formatCurrency(ada)}₳`;
}

/*
 * Code from: https://stackoverflow.com/a/18650828
 */
function formatBytes(bytes, decimals = 2) {
  if (bytes === 0) return '0 Bytes';

  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['Bytes', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB', 'EiB', 'ZiB', 'YiB'];

  const i = Math.floor(Math.log(bytes) / Math.log(k));

  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
}

function setChartJsDefaults(chartJs) {
  const backgroundColor =
    [
      'rgba(255, 99, 132, 0.2)',
      'rgba(255, 159, 64, 0.2)',
      'rgba(255, 205, 86, 0.2)',
      'rgba(75, 192, 192, 0.2)',
      'rgba(54, 162, 235, 0.2)',
      'rgba(153, 102, 255, 0.2)',
      'rgba(201, 203, 207, 0.2)'
    ]
  const borderColor = [
    'rgb(255, 99, 132)',
    'rgb(255, 159, 64)',
    'rgb(255, 205, 86)',
    'rgb(75, 192, 192)',
    'rgb(54, 162, 235)',
    'rgb(153, 102, 255)',
    'rgb(201, 203, 207)'
  ];

  // Global - hide charts title
  chartJs.defaults.plugins.legend.display = false;

  // Pie chart
  chartJs.defaults.elements.arc.backgroundColor = backgroundColor;
  chartJs.defaults.elements.arc.borderColor = borderColor;
  chartJs.defaults.elements.arc.borderWidth = 1;

  // Bar chart
  chartJs.defaults.elements.bar.backgroundColor = backgroundColor;
  chartJs.defaults.elements.bar.borderColor = borderColor;
  chartJs.defaults.elements.bar.borderWidth = 1;
}

module.exports = {
  checkUrl,
  formatStake,
  setChartJsDefaults,
  toAda,
  formatCurrency,
  formatBytes,
  formatPartyId,
}