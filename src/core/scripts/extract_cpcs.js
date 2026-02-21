// Script to extract CPCs from summary-box using data-cpc attribute
(() => {
    const items = [];
    const stateModifiers = document.querySelectorAll('state-modifier[data-cpc]');

    for (const modifier of stateModifiers) {
        const name = modifier.getAttribute('data-cpc');
        // Find the percentage value - it's in a sibling .value element
        const nameBlock = modifier.closest('.nameblock');
        let percentage = '';
        if (nameBlock) {
            const valueEl = nameBlock.querySelector('.value');
            if (valueEl) {
                percentage = valueEl.textContent.trim();
            }
        }
        if (name) {
            items.push({ name, percentage });
        }
    }

    return items.length > 0 ? items : null;
})()
