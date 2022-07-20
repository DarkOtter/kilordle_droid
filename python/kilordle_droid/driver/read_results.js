(function (results_elem) {
    const colourMatch = new Map();
    colourMatch.set("rgb(34, 238, 51)", "O")
    colourMatch.set("rgb(238, 222, 35)", "o")

    const classMatch = new Map();
    for (const div of results_elem.getElementsByTagName('div')) {
        const className = div.className;
        if (classMatch.has(className)) {
            continue;
        }
        const style = window.getComputedStyle(div);
        const colour = style["background-color"];
        let match = colourMatch.get(colour);
        if (match == undefined) {
            match = " ";
        }
        classMatch.set(className, match);
    }

    const columnResults = [];
    for (const column_elem of results_elem.children) {
        if (column_elem.textContent.includes("+")) {
            continue;
        }

        const rowDivs = []
        for (let i = 0; i < column_elem.children.length; i++) {
            const child = column_elem.children[i]
            if (child.tagName != "DIV") {
                throw new Error("Column has a child which is not a div")
            }
            rowDivs.push(child)
        }
        if (rowDivs.length < 1) {
            throw new Error("Did not find any rows")
        } else if (rowDivs[rowDivs.length - 1].textContent.trim() !== "") {
            throw new Error("Last row is not empty")
        }
        rowDivs.pop();
        let guessHistory = []
        let resultHistory = []
        for (const row of rowDivs) {
            if (row.children.length != 5) {
                throw new Error("Row does not have five elements")
            }
            let guess = ""
            let results = ""
            for (let i = 0; i < 5; ++i) {
                const child = row.children[i];
                guess += child.textContent;
                results += classMatch.get(child.className);
            }
            guessHistory.push(guess);
            resultHistory.push(results);
        }

        columnResults.push({ guessHistory, resultHistory })
    }
    return columnResults
})(...arguments);
