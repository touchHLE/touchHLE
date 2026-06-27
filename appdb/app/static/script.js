// Client-side enhancements: localised <time> formatting, sortable / searchable
// tables (matches the behaviour of appdb.touchhle.org).
(function () {
  "use strict";

  // Convert <time datetime> contents to the user's local timezone using a
  // sortable ISO-ish format, and tooltip the original UTC value.
  function processDateTime(time) {
    var raw = time.getAttribute("datetime");
    if (!raw) return;
    var date = new Date(raw);
    if (isNaN(date.getTime())) return;
    function pad(n) { return ("00" + n).slice(-2); }
    var local = date.getFullYear() + "-" + pad(date.getMonth() + 1) + "-" +
                pad(date.getDate()) + " " + pad(date.getHours()) + ":" +
                pad(date.getMinutes()) + ":" + pad(date.getSeconds());
    time.textContent = local;
    time.title = raw;
  }

  // Wrap a table in a "Search:" filter (rows matching the substring stay
  // visible, others are hidden).
  function attachSearch(table) {
    var label = document.createElement("label");
    label.className = "table-search";
    label.appendChild(document.createTextNode("Search: "));
    var input = document.createElement("input");
    input.type = "search";
    input.placeholder = "filter rows…";
    label.appendChild(input);
    var rows = Array.prototype.slice.call(table.tBodies[0].rows);
    input.oninput = function () {
      var q = input.value.toLowerCase();
      for (var i = 0; i < rows.length; i++) {
        var match = !q || rows[i].textContent.toLowerCase().indexOf(q) !== -1;
        rows[i].style.display = match ? "" : "none";
      }
    };
    table.parentElement.insertBefore(label, table);
  }

  // Make a table's column headers clickable for sorting. Click cycles
  // ascending → descending → unsorted, like the original appdb.
  function attachSorting(table) {
    var thead = table.tHead;
    if (!thead || !thead.rows.length) return;
    var headerRow = thead.rows[0];
    var rows = Array.prototype.slice.call(table.tBodies[0].rows);

    var sortBy = null;
    var sortAscending = null;
    var columns = [];

    function rowKey(row, idx) {
      var cell = row.cells[idx];
      if (!cell) return "";
      // Prefer a <time datetime> child for date columns so we sort
      // chronologically rather than alphabetically.
      var t = cell.querySelector("time[datetime]");
      if (t) return t.getAttribute("datetime") || "";
      // Star ratings: count "★"/⭐ characters so 5 stars > 1 star.
      if (cell.classList.contains("stars")) {
        var stars = cell.textContent.match(/[\u2605\u2B50\u2606]/g);
        return (stars ? stars.length : 0).toString().padStart(3, "0");
      }
      // Numeric columns: pad to 12 chars so lexicographic compare matches numeric.
      var raw = cell.textContent.trim();
      if (/^\d+$/.test(raw)) return raw.padStart(12, "0");
      return raw.toLowerCase();
    }

    function applySort() {
      var sorted;
      if (sortBy === null) {
        sorted = rows.slice();
      } else {
        sorted = rows.slice().sort(function (a, b) {
          var ak = rowKey(a, sortBy);
          var bk = rowKey(b, sortBy);
          var cmp = ak < bk ? -1 : ak > bk ? 1 : 0;
          return sortAscending ? cmp : -cmp;
        });
      }
      var tbody = table.tBodies[0];
      for (var i = 0; i < sorted.length; i++) {
        tbody.appendChild(sorted[i]);
      }
    }

    function refreshIndicators() {
      for (var i = 0; i < columns.length; i++) {
        var btn = columns[i].button;
        if (sortBy === i) {
          btn.textContent = sortAscending ? "▲" : "▼";
          btn.title = sortAscending
            ? "Click to sort by this column (descending)"
            : "Click to stop sorting";
        } else {
          btn.textContent = "↕";
          btn.title = "Click to sort by this column (ascending)";
        }
      }
    }

    for (var i = 0; i < headerRow.cells.length; i++) {
      (function (idx) {
        var th = headerRow.cells[idx];
        // Wrap original header text and add a sort button.
        var wrap = document.createElement("span");
        wrap.className = "sortable-column-header";
        var text = document.createElement("span");
        text.className = "sortable-column-text";
        while (th.firstChild) text.appendChild(th.firstChild);
        var btn = document.createElement("button");
        btn.type = "button";
        btn.className = "sortable-column-button";
        btn.textContent = "↕";
        btn.title = "Click to sort by this column (ascending)";
        btn.onclick = function () {
          if (sortBy === idx) {
            if (sortAscending) {
              sortAscending = false;
            } else {
              sortBy = null;
              sortAscending = null;
            }
          } else {
            sortBy = idx;
            sortAscending = true;
          }
          refreshIndicators();
          applySort();
        };
        wrap.appendChild(btn);
        wrap.appendChild(document.createTextNode(" "));
        wrap.appendChild(text);
        th.appendChild(wrap);
        columns.push({ element: th, button: btn });
      }(i));
    }
    refreshIndicators();
  }

  document.addEventListener("DOMContentLoaded", function () {
    var times = document.querySelectorAll("time[datetime]");
    for (var i = 0; i < times.length; i++) processDateTime(times[i]);

    var tables = document.querySelectorAll("table.sortable-table");
    for (var j = 0; j < tables.length; j++) {
      attachSorting(tables[j]);
      if (tables[j].classList.contains("searchable-table")) {
        attachSearch(tables[j]);
      }
    }
  });
}());
