function dragOverHandler(event) {
  // Prevent default behavior to allow drop
  event.preventDefault();
}

function dropHandler(event) {
  // Prevent default behavior to open dropped content as a link
  event.preventDefault();
  // Get the dropped data, which is a DataTransfer object
  const dataTransfer = event.dataTransfer;
  // Check if the data type is 'text' or 'text/plain'
  if (dataTransfer.types.includes('text/plain')) {
    // Get the dropped text and do something with it
    const droppedText = dataTransfer.getData('text/plain');
    // You can also perform other actions with the dropped text as needed
    appendCalendarLink(droppedText);
  }
}


function appendCalendarLink(url) {
  var searchParams = new URLSearchParams(window.location.search);
  var param = searchParams.get("q")
  if (param != null && param != "") {
    param = param.split(",");
    param.push(url);
  } else {
    param = url;
  }

  window.location.search = 'q=' + param;
}

let db = {events: [], cached_dates: {}};

var addToCache = (events, q, start, end) => { 
  db.events.push(...events);
  if (!db.cached_dates[q]) {
    db.cached_dates[q] = [];
  }
  intervals = db.cached_dates[q];
  intervals.push([start, end]);
  
  // merge the list of start,end date intervals compressing overlapping intervals
  intervals.sort((a, b) => a[0] - b[0]);
  for (let i = 0; i < intervals.length - 1; i++) {
    if (intervals[i][1] >= intervals[i + 1][0]) {
      intervals[i][1] = max(intervals[i][1], intervals[i + 1][1]);
      intervals.splice(i + 1, 1);
      i--;
    }
  }
}

var max = (a,b) => { 
  if (a > b) {
    return a;
  }
  return b;
}

var getFromCache = (q, start, end) => {
  if (!db.cached_dates[q]) {
    return null;
  }
  for (let i = 0; i < db.cached_dates[q].length; i++) {
    if (db.cached_dates[q][i][0] <= start && db.cached_dates[q][i][1] >= end) {
      return db.events;
    }
  }
  
  return null;
};

function getEvents(info, successCallback, failureCallback ) { 
  const handleErrors = response => {
    if (!response.ok) {
      failureCallback(err);
    }
    return response;
  };

	const queryString = window.location.search;
	const urlParams = new URLSearchParams(queryString);
	const q = urlParams.get('q');

  if (!q) {
    successCallback([]);
    return;
  }

  let cached_events = getFromCache(q, info.startStr, info.endStr);
  if (cached_events) {
    successCallback(cached_events);
    return;
  }

  const params = new URLSearchParams({ q: q, start: info.startStr, end: info.endStr});
  fetch(`/findtime?` + params, { method: 'GET'})
    .then(response => response.json())
    .then(res => {
        addToCache(res, q, info.startStr, info.endStr);
        successCallback(res);
    });
}


function isQueryParamMissing() {
  const urlParams = new URLSearchParams(window.location.search);
  return !urlParams.has('q') || urlParams.get('q') == "";
}

window.onload = function() {
  if (isQueryParamMissing()) {
      document.getElementById('drop-zone').classList.add('disabled');
  }
};

async function clipboardReadHandler() {
  try {
    // Check if the Clipboard API is available
    if (!navigator.clipboard) {
      console.log('Clipboard API not available');
      return;
    }

    // Try to read the text from the clipboard
    const text = await navigator.clipboard.readText();
    appendCalendarLink(text);
    console.log('Pasted content: ', text);
  } catch (err) {
    console.error('Failed to read clipboard contents: ', err);
  }
}