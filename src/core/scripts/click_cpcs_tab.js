// Script to click CPCs tab and click Expand
(() => {
    function findElementByText(text, root = document) {
        const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, null, false);
        let node;
        while (node = walker.nextNode()) {
            if (node.textContent.trim() === text) {
                return node.parentElement;
            }
        }
        const allElements = root.querySelectorAll('*');
        for (const el of allElements) {
            if (el.shadowRoot) {
                const found = findElementByText(text, el.shadowRoot);
                if (found) return found;
            }
        }
        return null;
    }

    // Click CPCs tab
    const cpcsTab = findElementByText('CPCs');
    if (cpcsTab) {
        cpcsTab.click();
    }

    // Click Expand button for CPCs after a small delay
    const expandBtn = findElementByText('Expand');
    if (expandBtn) {
        expandBtn.click();
    }

    return { clicked: cpcsTab !== null };
})()
