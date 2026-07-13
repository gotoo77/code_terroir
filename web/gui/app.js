function switchTab(tabName) {
    document.querySelectorAll('.tab-content').forEach((tab) => tab.classList.remove('active'));
    document.querySelectorAll('.tab').forEach((tab) => tab.classList.remove('active'));

    document.getElementById(`${tabName}-tab`).classList.add('active');
    if (event && event.target) {
        event.target.classList.add('active');
    }
}

function showResponse(responseId, status, statusText, body, isError = false) {
    const responseDiv = document.getElementById(responseId);
    const responseBody = responseDiv.querySelector('.response-body');
    const statusClass = isError
        ? 'status-500'
        : status >= 500
            ? 'status-500'
            : status >= 400
                ? 'status-400'
                : 'status-200';

    responseDiv.style.display = 'block';
    const statusLine = document.createElement('div');
    statusLine.className = statusClass;
    statusLine.textContent = `Status: ${status} ${statusText}`;
    const label = document.createElement('div');
    label.textContent = 'Response Body:';
    const content = document.createElement('pre');
    content.style.marginLeft = '20px';
    content.style.whiteSpace = 'pre-wrap';
    content.textContent = body;
    responseBody.replaceChildren(statusLine, document.createElement('br'), label, content);
}

function apiHeaders(extraHeaders = {}) {
    const headers = { 'Content-Type': 'application/json', ...extraHeaders };
    const accessToken = sessionStorage.getItem('accessToken');
    if (accessToken) {
        headers.Authorization = `Bearer ${accessToken}`;
    }
    return headers;
}

function rememberTokens(endpoint, response, data) {
    if (!response.ok || !['/api/v1/auth/login', '/api/v1/auth/refresh', '/api/v1/auth/register'].includes(endpoint)) {
        return;
    }
    try {
        const payload = JSON.parse(data);
        if (payload.access_token) sessionStorage.setItem('accessToken', payload.access_token);
        if (payload.refresh_token) sessionStorage.setItem('refreshToken', payload.refresh_token);
    } catch (_error) {
        // La réponse brute reste affichée si elle n'est pas au format JSON attendu.
    }
}

async function callApi(responseId, method, endpoint, body = null, extraHeaders = {}) {
    const responseDiv = document.getElementById(responseId);
    const responseBody = responseDiv.querySelector('.response-body');
    responseDiv.style.display = 'block';
    const loading = document.createElement('div');
    loading.className = 'loading';
    loading.textContent = 'Chargement...';
    responseBody.replaceChildren(loading);

    try {
        const response = await fetch(`${API_BASE}${endpoint}`, {
            method,
            headers: apiHeaders(extraHeaders),
            body: body ? JSON.stringify(body) : null,
        });
        const data = await response.text();
        rememberTokens(endpoint, response, data);
        showResponse(responseId, response.status, response.statusText, data);
        return response.status;
    } catch (error) {
        showResponse(responseId, 500, 'Erreur de connexion', error.message, true);
        return 500;
    }
}

async function makeRequest(method, endpoint, body = null) {
    const map = {
        '/health': 'response-health',
        '/api/v1/producers': 'response-apiv1producers',
        '/api/v1/products': 'response-apiv1products',
        '/api/v1/batches': 'response-apiv1batches',
        '/api/v1/suppliers': 'response-apiv1suppliers',
        '/api/v1/ingredients': 'response-apiv1ingredients',
        '/api/v1/recipes': 'response-apiv1recipes',
        '/api/v1/qa-checks': 'response-apiv1qachecks',
        '/api/v1/qr': 'response-apiv1qr',
    };
    const responseId = map[endpoint];
    if (responseId) {
        await callApi(responseId, method, endpoint, body);
    }
}

async function testAuthLogin() {
    const payload = {
        email: document.getElementById('auth-email').value.trim() || null,
        password: document.getElementById('auth-password').value.trim() || null,
        totp_code: null,
    };
    await callApi('response-auth-login', 'POST', '/api/v1/auth/login', payload);
}

async function testAuthRegister() {
    const producerFields = {
        raison_sociale: document.getElementById('auth-producer-name').value.trim(),
        adresse: document.getElementById('auth-producer-address').value.trim(),
        code_postal: document.getElementById('auth-producer-postal-code').value.trim(),
        ville: document.getElementById('auth-producer-city').value.trim(),
        pays: document.getElementById('auth-producer-country').value.trim() || 'France',
    };
    const hasProducerDetails = Object.values(producerFields).some((value) => value.length > 0);
    const payload = {
        producer_id: document.getElementById('auth-producer-id').value.trim() || null,
        producer: hasProducerDetails ? producerFields : null,
        email: document.getElementById('auth-email').value.trim() || null,
        password: document.getElementById('auth-password').value.trim() || null,
        first_name: document.getElementById('auth-first-name').value.trim() || null,
        last_name: document.getElementById('auth-last-name').value.trim() || null,
    };
    const bootstrapToken = document.getElementById('auth-bootstrap-token').value;
    await callApi('response-auth-register', 'POST', '/api/v1/auth/register', payload, {
        'X-Bootstrap-Token': bootstrapToken,
    });
}

async function testAuthRefresh() {
    await callApi('response-auth-refresh', 'POST', '/api/v1/auth/refresh', {
        refresh_token: document.getElementById('auth-refresh-token').value.trim() || null,
    });
}

function parseCommaList(value) {
    return value.split(',').map((item) => item.trim()).filter(Boolean);
}

function parseJsonOrNull(value) {
    const trimmed = value.trim();
    if (!trimmed) {
        return null;
    }
    return JSON.parse(trimmed);
}

function getMultiSelectValues(elementId) {
    return Array.from(document.getElementById(elementId).selectedOptions).map((option) => option.value);
}

function setMultiSelectValues(elementId, values) {
    const expected = new Set(values);
    Array.from(document.getElementById(elementId).options).forEach((option) => {
        option.selected = expected.has(option.value);
    });
}

const productCompositionState = {
    create: [],
    update: [],
};

const productCatalogState = {
    create: [],
    update: [],
};

function getProductCompositionIds(mode) {
    const isUpdate = mode === 'update';
    return {
        category: isUpdate ? 'product-update-composition-category' : 'product-composition-category',
        summary: isUpdate ? 'product-update-catalog-summary' : 'product-catalog-summary',
        suggestions: isUpdate ? 'product-update-catalog-suggestions' : 'product-catalog-suggestions',
        list: isUpdate ? 'product-update-composition-list' : 'product-composition-list',
        payload: isUpdate ? 'product-update-ingredients-json' : 'product-ingredients-json',
    };
}

async function loadProductCatalogSuggestions(mode) {
    const ids = getProductCompositionIds(mode);
    const category = document.getElementById(ids.category).value;
    if (!category) {
        return alert('Veuillez choisir une categorie d ingredient');
    }

    const summary = document.getElementById(ids.summary);
    const suggestions = document.getElementById(ids.suggestions);
    summary.style.display = 'block';
    summary.textContent = 'Chargement des suggestions...';
    suggestions.replaceChildren();

    try {
        const response = await fetch(`${API_BASE}/api/v1/catalog/categories/${encodeURIComponent(category)}/ingredients`, {
            headers: apiHeaders(),
        });
        const payload = await response.json();
        if (!response.ok || !payload.success) {
            summary.textContent = payload.error || 'Erreur de chargement des suggestions';
            return;
        }

        productCatalogState[mode] = payload.ingredients || [];
        summary.textContent = `${payload.count} suggestion(s) trouvee(s) pour la categorie ${payload.category} et ${payload.producers.length} producteur(s) associe(s).`;
        renderProductCatalogSuggestions(mode);
    } catch (error) {
        summary.textContent = `Erreur de chargement: ${error.message}`;
    }
}

function renderProductCatalogSuggestions(mode) {
    const ids = getProductCompositionIds(mode);
    const suggestions = document.getElementById(ids.suggestions);
    const items = productCatalogState[mode];
    suggestions.replaceChildren();

    if (!items.length) {
        const emptyState = document.createElement('div');
        emptyState.className = 'empty-state';
        emptyState.textContent = 'Aucune suggestion pour cette categorie.';
        suggestions.append(emptyState);
        return;
    }

    items.forEach((item, index) => {
        const card = document.createElement('div');
        card.className = 'catalog-card';
        const information = document.createElement('div');
        const name = document.createElement('strong');
        name.textContent = item.ingredient_name;
        information.append(name);

        const lines = [
            `Categorie: ${item.ingredient_category || 'non renseignee'}`,
            `Producteur: ${item.producer.producer_name} (${item.producer.producer_city})`,
            `Allergenes: ${item.ingredient_allergens.length ? item.ingredient_allergens.join(', ') : 'aucun'}`,
        ];
        lines.forEach((text) => {
            const line = document.createElement('div');
            line.className = 'muted-line';
            line.textContent = text;
            information.append(line);
        });

        const button = document.createElement('button');
        button.type = 'button';
        button.className = 'btn';
        button.textContent = 'Ajouter';
        button.addEventListener('click', () => addCatalogSuggestionToComposition(mode, index));
        card.append(information, button);
        suggestions.append(card);
    });
}

function addCatalogSuggestionToComposition(mode, index) {
    const suggestion = productCatalogState[mode][index];
    if (!suggestion) {
        return;
    }

    productCompositionState[mode].push({
        ingredient_id: suggestion.ingredient_id,
        producer_id: suggestion.producer.producer_id,
        ingredient_category: suggestion.ingredient_category,
        quantity: null,
        unit: '%',
        notes: null,
        sort_order: productCompositionState[mode].length,
        label: `${suggestion.ingredient_name} -> ${suggestion.producer.producer_name}`,
    });

    renderProductComposition(mode);
}

function updateCompositionField(mode, index, field, value) {
    const item = productCompositionState[mode][index];
    if (!item) {
        return;
    }

    if (field === 'quantity') {
        item[field] = value === '' ? null : parseFloat(value);
    } else if (field === 'sort_order') {
        item[field] = value === '' ? index : parseInt(value, 10);
    } else {
        item[field] = value || null;
    }

    renderProductComposition(mode);
}

function removeCompositionItem(mode, index) {
    productCompositionState[mode].splice(index, 1);
    productCompositionState[mode].forEach((item, currentIndex) => {
        item.sort_order = currentIndex;
    });
    renderProductComposition(mode);
}

function renderProductComposition(mode) {
    const ids = getProductCompositionIds(mode);
    const container = document.getElementById(ids.list);
    const payload = document.getElementById(ids.payload);
    const items = productCompositionState[mode];

    payload.value = JSON.stringify(items.map(({ label, ...item }) => item), null, 2);
    container.replaceChildren();

    if (!items.length) {
        const emptyState = document.createElement('div');
        emptyState.className = 'empty-state';
        emptyState.textContent = 'Aucun ingredient ajoute pour le moment.';
        container.append(emptyState);
        return;
    }

    items.forEach((item, index) => {
        const card = document.createElement('div');
        card.className = 'composition-item';
        const header = document.createElement('div');
        header.className = 'composition-item-header';
        const title = document.createElement('strong');
        title.textContent = item.label;
        const removeButton = document.createElement('button');
        removeButton.type = 'button';
        removeButton.className = 'btn danger-btn';
        removeButton.textContent = 'Supprimer';
        removeButton.addEventListener('click', () => removeCompositionItem(mode, index));
        header.append(title, removeButton);

        const grid = document.createElement('div');
        grid.className = 'form-grid-4';
        grid.append(
            createCompositionField('Quantite', 'number', item.quantity ?? '', { step: '0.1' }, (value) => updateCompositionField(mode, index, 'quantity', value)),
            createCompositionField('Unite', 'text', item.unit ?? '', {}, (value) => updateCompositionField(mode, index, 'unit', value)),
            createCompositionField('Ordre', 'number', item.sort_order ?? index, { min: '0' }, (value) => updateCompositionField(mode, index, 'sort_order', value)),
            createCompositionField('Categorie', 'text', item.ingredient_category ?? '', {}, (value) => updateCompositionField(mode, index, 'ingredient_category', value)),
        );
        const notes = createCompositionField('Notes', 'text', item.notes ?? '', {}, (value) => updateCompositionField(mode, index, 'notes', value));
        card.append(header, grid, notes);
        container.append(card);
    });
}

function createCompositionField(labelText, type, value, attributes, onChange) {
    const group = document.createElement('div');
    group.className = 'form-group';
    const label = document.createElement('label');
    label.textContent = labelText;
    const input = document.createElement('input');
    input.type = type;
    input.value = value;
    Object.entries(attributes).forEach(([name, attributeValue]) => input.setAttribute(name, attributeValue));
    input.addEventListener('change', () => onChange(input.value));
    group.append(label, input);
    return group;
}

async function getProductById() {
    const productId = document.getElementById('product-id').value.trim();
    if (!productId) return alert('Veuillez saisir un ID de produit');
    await callApi('response-get-product', 'GET', `/api/v1/products/${productId}`);
}

async function getProducerById() {
    const producerId = document.getElementById('producer-id').value.trim();
    if (!producerId) return alert('Veuillez saisir un ID de producteur');
    await callApi('response-get-producer', 'GET', `/api/v1/producers/${producerId}`);
}

function fillExampleProducer() {
    document.getElementById('producer-raison-sociale').value = 'Apothicaire Culinaire';
    document.getElementById('producer-agrement').value = 'FR-47-000-EC';
    document.getElementById('producer-siret').value = '12345678901234';
    document.getElementById('producer-adresse').value = '12 rue des producteurs';
    document.getElementById('producer-code-postal').value = '47000';
    document.getElementById('producer-ville').value = 'Agen';
    document.getElementById('producer-pays').value = 'France';
    document.getElementById('producer-email').value = 'contact@apothicaire.example.com';
    document.getElementById('producer-telephone').value = '+33 5 12 34 56 78';
    document.getElementById('producer-site-web').value = 'https://apothicaire.example.com';
    document.getElementById('producer-logo-url').value = 'https://example.com/logo.png';
    document.getElementById('producer-photo-url').value = 'https://example.com/photo-atelier.jpg';
    document.getElementById('producer-categorie-principale').value = 'legumes';
}

async function createProducer() {
    const raison_sociale = document.getElementById('producer-raison-sociale').value.trim();
    const adresse = document.getElementById('producer-adresse').value.trim();
    const code_postal = document.getElementById('producer-code-postal').value.trim();
    const ville = document.getElementById('producer-ville').value.trim();
    const pays = document.getElementById('producer-pays').value.trim();
    const email = document.getElementById('producer-email').value.trim();
    if (!raison_sociale || !adresse || !code_postal || !ville || !pays || !email) {
        return alert('Raison sociale, adresse, code postal, ville, pays et email sont obligatoires');
    }

    const payload = {
        raison_sociale,
        agrement_sanitaire: document.getElementById('producer-agrement').value.trim() || null,
        siret: document.getElementById('producer-siret').value.trim() || null,
        adresse,
        code_postal,
        ville,
        pays,
        email,
        telephone: document.getElementById('producer-telephone').value.trim() || null,
        site_web: document.getElementById('producer-site-web').value.trim() || null,
        logo_url: document.getElementById('producer-logo-url').value.trim() || null,
        photo_url: document.getElementById('producer-photo-url').value.trim() || null,
        categorie_principale: document.getElementById('producer-categorie-principale').value || null,
    };
    await callApi('response-create-producer', 'POST', '/api/v1/producers', payload);
}

async function updateProducer() {
    const producerId = document.getElementById('producer-update-id').value.trim();
    if (!producerId) return alert('Veuillez saisir un ID de producteur');

    const payload = {
        raison_sociale: document.getElementById('producer-update-raison-sociale').value.trim() || null,
        agrement_sanitaire: document.getElementById('producer-update-agrement').value.trim() || null,
        siret: document.getElementById('producer-update-siret').value.trim() || null,
        adresse: document.getElementById('producer-update-adresse').value.trim() || null,
        code_postal: document.getElementById('producer-update-code-postal').value.trim() || null,
        ville: document.getElementById('producer-update-ville').value.trim() || null,
        pays: document.getElementById('producer-update-pays').value.trim() || null,
        email: document.getElementById('producer-update-email').value.trim() || null,
        telephone: document.getElementById('producer-update-telephone').value.trim() || null,
        site_web: document.getElementById('producer-update-site-web').value.trim() || null,
        logo_url: document.getElementById('producer-update-logo-url').value.trim() || null,
        photo_url: document.getElementById('producer-update-photo-url').value.trim() || null,
        categorie_principale: document.getElementById('producer-update-categorie-principale').value || null,
    };
    await callApi('response-update-producer', 'PUT', `/api/v1/producers/${producerId}`, payload);
}

function fillExampleProduct() {
    document.getElementById('product-producer-id').value = '550e8400-e29b-41d4-a716-446655440001';
    document.getElementById('product-name').value = 'Veloute des Maraichers';
    document.getElementById('product-category').value = 'sauce';
    document.getElementById('product-description').value = 'Une recette douce et vive, cuisinee en petite serie avec les legumes de saison.';
    document.getElementById('product-image-url').value = 'https://images.unsplash.com/photo-1547592180-85f173990554?auto=format&fit=crop&w=900&q=80';
    document.getElementById('product-nutriscore').value = 'B';
    document.getElementById('product-energy').value = '118';
    document.getElementById('product-carbs').value = '12';
    document.getElementById('product-fat').value = '6';
    document.getElementById('product-proteins').value = '4.8';
    document.getElementById('product-ingredients-json').value = '[{\"ingredient_id\":\"550e8400-e29b-41d4-a716-446655440020\",\"producer_id\":\"550e8400-e29b-41d4-a716-446655440001\",\"quantity\":70,\"unit\":\"%\",\"ingredient_category\":\"legumes\"}]';
    setMultiSelectValues('product-allergens', ['lait']);
    document.getElementById('product-labels').value = 'Artisanal, Lot-et-Garonne';
    document.getElementById('product-usage').value = 'Servir chaud avec quelques graines torrefiees.';
    document.getElementById('product-legal').value = 'Valeurs nutritionnelles pour 100 g. Conserver au frais apres ouverture.';
}

async function createProduct() {
    const name = document.getElementById('product-name').value.trim();
    const category = document.getElementById('product-category').value;
    if (!name || !category) return alert('Le nom et la categorie du produit sont obligatoires');

    let product_ingredients = null;
    try {
        product_ingredients = parseJsonOrNull(document.getElementById('product-ingredients-json').value) || [];
    } catch (error) {
        return alert(`JSON composition produit invalide: ${error.message}`);
    }

    const payload = {
        name,
        category,
        producer_id: document.getElementById('product-producer-id').value.trim() || null,
        description_marketing: document.getElementById('product-description').value.trim() || null,
        image_url: document.getElementById('product-image-url').value.trim() || null,
        nutriscore: document.getElementById('product-nutriscore').value || null,
        energie_kcal_100g: document.getElementById('product-energy').value ? parseFloat(document.getElementById('product-energy').value) : null,
        glucides_100g: document.getElementById('product-carbs').value ? parseFloat(document.getElementById('product-carbs').value) : null,
        lipides_100g: document.getElementById('product-fat').value ? parseFloat(document.getElementById('product-fat').value) : null,
        proteines_100g: document.getElementById('product-proteins').value ? parseFloat(document.getElementById('product-proteins').value) : null,
        allergenes: getMultiSelectValues('product-allergens'),
        product_ingredients,
        labels_certifications: parseCommaList(document.getElementById('product-labels').value),
        conseils_utilisation: document.getElementById('product-usage').value.trim() || null,
        infos_legales: document.getElementById('product-legal').value.trim() || null,
    };
    await callApi('response-create-product', 'POST', '/api/v1/products', payload);
}

async function updateProduct() {
    const productId = document.getElementById('product-update-id').value.trim();
    if (!productId) return alert('Veuillez saisir un ID de produit');

    const labels = document.getElementById('product-update-labels').value.trim();
    let product_ingredients = null;
    try {
        product_ingredients = parseJsonOrNull(document.getElementById('product-update-ingredients-json').value);
    } catch (error) {
        return alert(`JSON composition produit invalide: ${error.message}`);
    }

    const payload = {
        name: document.getElementById('product-update-name').value.trim() || null,
        category: document.getElementById('product-update-category').value || null,
        producer_id: document.getElementById('product-update-producer-id').value.trim() || null,
        description_marketing: document.getElementById('product-update-description').value.trim() || null,
        image_url: document.getElementById('product-update-image-url').value.trim() || null,
        nutriscore: document.getElementById('product-update-nutriscore').value || null,
        energie_kcal_100g: document.getElementById('product-update-energy').value ? parseFloat(document.getElementById('product-update-energy').value) : null,
        glucides_100g: document.getElementById('product-update-carbs').value ? parseFloat(document.getElementById('product-update-carbs').value) : null,
        lipides_100g: document.getElementById('product-update-fat').value ? parseFloat(document.getElementById('product-update-fat').value) : null,
        proteines_100g: document.getElementById('product-update-proteins').value ? parseFloat(document.getElementById('product-update-proteins').value) : null,
        allergenes: getMultiSelectValues('product-update-allergens').length ? getMultiSelectValues('product-update-allergens') : null,
        product_ingredients,
        labels_certifications: labels ? parseCommaList(labels) : null,
        conseils_utilisation: document.getElementById('product-update-usage').value.trim() || null,
        infos_legales: document.getElementById('product-update-legal').value.trim() || null,
    };
    await callApi('response-update-product', 'PUT', `/api/v1/products/${productId}`, payload);
}

function fillExampleBatch() {
    document.getElementById('batch-product-id').value = '550e8400-e29b-41d4-a716-446655440100';
    const dluo = new Date();
    dluo.setMonth(dluo.getMonth() + 6);
    document.getElementById('batch-dluo').value = dluo.toISOString().slice(0, 16);
    document.getElementById('batch-quantity').value = '240';
    document.getElementById('batch-site').value = 'Atelier Principal - Zone A';
    document.getElementById('batch-operators').value = 'Jean Dupont, Marie Martin';
    document.getElementById('batch-notes').value = 'Lot de production automne 2025.';
}

async function getBatchById() {
    const batchId = document.getElementById('batch-id').value.trim();
    if (!batchId) return alert('Veuillez saisir un ID de lot');
    await callApi('response-get-batch', 'GET', `/api/v1/batches/${batchId}`);
}

async function createBatch() {
    const productId = document.getElementById('batch-product-id').value.trim();
    const dluo = document.getElementById('batch-dluo').value;
    const quantity = document.getElementById('batch-quantity').value.trim();
    const site = document.getElementById('batch-site').value.trim();
    if (!productId || !dluo || !quantity || !site) return alert("L'ID du produit, la DLUO, la quantite et le site sont obligatoires");

    const payload = {
        product_id: productId,
        lot_code: document.getElementById('batch-lot-code').value.trim() || null,
        dluo_ddm: new Date(dluo).toISOString(),
        quantity_produced: parseInt(quantity, 10),
        production_site: site,
        operators: parseCommaList(document.getElementById('batch-operators').value),
        notes: document.getElementById('batch-notes').value.trim() || null,
    };
    await callApi('response-create-batch', 'POST', '/api/v1/batches', payload);
}

async function recallBatch() {
    const batchId = document.getElementById('recall-batch-id').value.trim();
    const reason = document.getElementById('recall-reason').value.trim();
    if (!batchId || !reason) return alert("L'ID du lot et la raison du rappel sont obligatoires");
    await callApi('response-recall-batch', 'POST', `/api/v1/batches/${batchId}/recall`, { reason });
}

async function updateBatch() {
    const batchId = document.getElementById('batch-update-id').value.trim();
    if (!batchId) return alert('Veuillez saisir un ID de lot');

    const payload = {
        dluo_ddm: document.getElementById('batch-update-dluo').value ? new Date(document.getElementById('batch-update-dluo').value).toISOString() : null,
        quantity_produced: document.getElementById('batch-update-quantity').value ? parseInt(document.getElementById('batch-update-quantity').value, 10) : null,
        production_site: document.getElementById('batch-update-site').value.trim() || null,
        operators: document.getElementById('batch-update-operators').value.trim() ? parseCommaList(document.getElementById('batch-update-operators').value) : null,
        notes: document.getElementById('batch-update-notes').value.trim() || null,
    };
    await callApi('response-update-batch', 'PUT', `/api/v1/batches/${batchId}`, payload);
}

async function getSupplierById() {
    const supplierId = document.getElementById('supplier-id').value.trim();
    if (!supplierId) return alert('Veuillez saisir un ID de fournisseur');
    await callApi('response-get-supplier', 'GET', `/api/v1/suppliers/${supplierId}`);
}

function fillExampleSupplier() {
    document.getElementById('supplier-producer-id').value = '550e8400-e29b-41d4-a716-446655440001';
    document.getElementById('supplier-name').value = 'Ferme du Soleil Levant';
    document.getElementById('supplier-email').value = 'contact@ferme-soleil.fr';
    document.getElementById('supplier-phone').value = '+33 5 12 34 56 78';
    document.getElementById('supplier-address').value = '123 Rue de la Campagne, 47000 Agen';
}

async function createSupplier() {
    const producer_id = document.getElementById('supplier-producer-id').value.trim();
    const name = document.getElementById('supplier-name').value.trim();
    if (!producer_id || !name) return alert('Le producer_id et le nom sont obligatoires');

    const payload = {
        producer_id,
        name,
        contact_email: document.getElementById('supplier-email').value.trim() || null,
        contact_phone: document.getElementById('supplier-phone').value.trim() || null,
        address: document.getElementById('supplier-address').value.trim() || null,
    };
    await callApi('response-create-supplier', 'POST', '/api/v1/suppliers', payload);
}

async function updateSupplier() {
    const supplierId = document.getElementById('supplier-update-id').value.trim();
    if (!supplierId) return alert('Veuillez saisir un ID de fournisseur');

    const payload = {
        name: document.getElementById('supplier-update-name').value.trim() || null,
        contact_email: document.getElementById('supplier-update-email').value.trim() || null,
        contact_phone: document.getElementById('supplier-update-phone').value.trim() || null,
        address: document.getElementById('supplier-update-address').value.trim() || null,
    };
    await callApi('response-update-supplier', 'PUT', `/api/v1/suppliers/${supplierId}`, payload);
}

async function getIngredientById() {
    const ingredientId = document.getElementById('ingredient-id').value.trim();
    if (!ingredientId) return alert("Veuillez saisir un ID d'ingredient");
    await callApi('response-get-ingredient', 'GET', `/api/v1/ingredients/${ingredientId}`);
}

function fillExampleIngredient() {
    document.getElementById('ingredient-producer-id').value = '550e8400-e29b-41d4-a716-446655440001';
    document.getElementById('ingredient-supplier-id').value = '550e8400-e29b-41d4-a716-446655440010';
    document.getElementById('ingredient-name').value = 'Carottes des Landes';
    document.getElementById('ingredient-category').value = 'legumes';
    setMultiSelectValues('ingredient-allergens', []);
    document.getElementById('ingredient-nutrition').value = '{"energie_kcal": 41, "glucides": 9.6, "lipides": 0.2, "proteines": 0.9, "per_100g": true}';
    document.getElementById('ingredient-documents').value = 'https://example.com/docs/carottes.pdf';
}

async function createIngredient() {
    const producer_id = document.getElementById('ingredient-producer-id').value.trim();
    const name = document.getElementById('ingredient-name').value.trim();
    if (!producer_id || !name) return alert("Le producer_id et le nom sont obligatoires");

    let nutritional_info = null;
    try {
        nutritional_info = parseJsonOrNull(document.getElementById('ingredient-nutrition').value);
    } catch (error) {
        return alert(`JSON nutrition invalide: ${error.message}`);
    }

    const payload = {
        producer_id,
        supplier_id: document.getElementById('ingredient-supplier-id').value.trim() || null,
        name,
        category: document.getElementById('ingredient-category').value || null,
        allergens: getMultiSelectValues('ingredient-allergens'),
        nutritional_info,
        documents: parseCommaList(document.getElementById('ingredient-documents').value),
    };
    await callApi('response-create-ingredient', 'POST', '/api/v1/ingredients', payload);
}

async function updateIngredient() {
    const ingredientId = document.getElementById('ingredient-update-id').value.trim();
    if (!ingredientId) return alert("Veuillez saisir un ID d'ingredient");

    let nutritional_info = null;
    try {
        nutritional_info = parseJsonOrNull(document.getElementById('ingredient-update-nutrition').value);
    } catch (error) {
        return alert(`JSON nutrition invalide: ${error.message}`);
    }

    const payload = {
        supplier_id: document.getElementById('ingredient-update-supplier-id').value.trim() || null,
        name: document.getElementById('ingredient-update-name').value.trim() || null,
        category: document.getElementById('ingredient-update-category').value || null,
        allergens: getMultiSelectValues('ingredient-update-allergens').length ? getMultiSelectValues('ingredient-update-allergens') : null,
        nutritional_info,
        documents: document.getElementById('ingredient-update-documents').value.trim() ? parseCommaList(document.getElementById('ingredient-update-documents').value) : null,
    };
    await callApi('response-update-ingredient', 'PUT', `/api/v1/ingredients/${ingredientId}`, payload);
}

async function getQACheckById() {
    const qaId = document.getElementById('qa-id').value.trim();
    if (!qaId) return alert('Veuillez saisir un ID de controle QA');
    await callApi('response-get-qa', 'GET', `/api/v1/qa-checks/${qaId}`);
}

async function listBatchQAChecks() {
    const batchId = document.getElementById('qa-batch-id-list').value.trim();
    if (!batchId) return alert('Veuillez saisir un ID de lot');
    await callApi('response-list-batch-qa', 'GET', `/api/v1/batches/${batchId}/qa`);
}

async function createQACheck() {
    const batchId = document.getElementById('qa-batch-id').value.trim();
    const operatorId = document.getElementById('qa-operator-id').value.trim();
    const checkType = document.getElementById('qa-check-type').value;
    if (!batchId || !operatorId || !checkType) return alert('Le lot, l operateur et le type de controle sont obligatoires');

    const payload = {
        operator_id: operatorId,
        check_type: checkType,
        value: document.getElementById('qa-value').value ? parseFloat(document.getElementById('qa-value').value) : null,
        min_threshold: document.getElementById('qa-min-threshold').value ? parseFloat(document.getElementById('qa-min-threshold').value) : null,
        max_threshold: document.getElementById('qa-max-threshold').value ? parseFloat(document.getElementById('qa-max-threshold').value) : null,
        unit: document.getElementById('qa-unit').value.trim() || null,
        notes: document.getElementById('qa-notes').value.trim() || null,
        attachments: document.getElementById('qa-attachments').value.trim() ? parseCommaList(document.getElementById('qa-attachments').value) : null,
    };
    await callApi('response-create-qa', 'POST', `/api/v1/batches/${batchId}/qa`, payload);
}

async function getBatchQASummary() {
    const batchId = document.getElementById('qa-batch-id-summary').value.trim();
    if (!batchId) return alert('Veuillez saisir un ID de lot');
    await callApi('response-batch-qa-summary', 'GET', `/api/v1/batches/${batchId}/qa/summary`);
}

async function getRecipeById() {
    const recipeId = document.getElementById('recipe-id').value.trim();
    if (!recipeId) return alert('Veuillez saisir un ID de recette');
    await callApi('response-get-recipe', 'GET', `/api/v1/recipes/${recipeId}`);
}

async function createRecipe() {
    const producer_id = document.getElementById('recipe-producer-id').value.trim();
    const name = document.getElementById('recipe-name').value.trim();
    if (!producer_id || !name) return alert('Le producer_id et le nom de recette sont obligatoires');

    let ingredients;
    let steps;
    try {
        ingredients = parseJsonOrNull(document.getElementById('recipe-ingredients').value) || [];
        steps = parseJsonOrNull(document.getElementById('recipe-steps').value) || [];
    } catch (error) {
        return alert(`JSON recette invalide: ${error.message}`);
    }

    const payload = {
        producer_id,
        name,
        ingredients,
        steps,
        notes: document.getElementById('recipe-notes').value.trim() || null,
    };
    await callApi('response-create-recipe', 'POST', '/api/v1/recipes', payload);
}

async function updateRecipe() {
    const recipeId = document.getElementById('recipe-update-id').value.trim();
    if (!recipeId) return alert('Veuillez saisir un ID de recette');

    let ingredients = null;
    let steps = null;
    try {
        ingredients = parseJsonOrNull(document.getElementById('recipe-update-ingredients').value);
        steps = parseJsonOrNull(document.getElementById('recipe-update-steps').value);
    } catch (error) {
        return alert(`JSON recette invalide: ${error.message}`);
    }

    const payload = {
        name: document.getElementById('recipe-update-name').value.trim() || null,
        ingredients,
        steps,
        notes: document.getElementById('recipe-update-notes').value.trim() || null,
        create_new_version: document.getElementById('recipe-update-new-version').checked,
    };
    await callApi('response-update-recipe', 'PUT', `/api/v1/recipes/${recipeId}`, payload);
}

function fillExampleQR() {
    document.getElementById('qr-batch-id').value = '550e8400-e29b-41d4-a716-446655440200';
    document.getElementById('qr-format').value = 'Both';
}

async function getQRById() {
    const qrId = document.getElementById('qr-id').value.trim();
    if (!qrId) return alert('Veuillez saisir un ID de QR code');
    await callApi('response-get-qr', 'GET', `/api/v1/qr/${qrId}`);
}

async function createQRTag() {
    const batch_id = document.getElementById('qr-batch-id').value.trim();
    if (!batch_id) return alert("L'ID du lot de production est obligatoire");
    await callApi('response-create-qr', 'POST', '/api/v1/qr', {
        batch_id,
        format: document.getElementById('qr-format').value,
    });
}

async function scanQRCode() {
    const slug = document.getElementById('qr-slug').value.trim();
    if (!slug) return alert('Le slug du QR code est obligatoire');
    await callApi('response-scan-qr', 'POST', `/api/v1/qr/scan/${slug}`, {
        ip_address: document.getElementById('scan-ip').value.trim() || null,
        user_agent: document.getElementById('scan-user-agent').value.trim() || 'Code Terroir GUI Test',
        referer: window.location.href,
    });
}

async function getQRAnalytics() {
    const qrId = document.getElementById('analytics-qr-id').value.trim();
    if (!qrId) return alert('Veuillez saisir un ID de QR code');
    await callApi('response-qr-analytics', 'GET', `/api/v1/qr/${qrId}/analytics`);
}

async function getPublicTraceability() {
    const slug = document.getElementById('public-slug').value.trim();
    if (!slug) return alert('Veuillez saisir un slug');
    await callApi('response-public-trace', 'GET', `/api/trace/${slug}`);
}

async function recordPublicVisit() {
    const slug = document.getElementById('public-visit-slug').value.trim();
    if (!slug) return alert('Veuillez saisir un slug');
    await callApi('response-public-visit', 'POST', `/api/trace/${slug}/visit`, {
        ip_address: document.getElementById('public-ip').value.trim() || null,
        user_agent: document.getElementById('public-user-agent').value.trim() || null,
        referer: document.getElementById('public-referer').value.trim() || null,
    });
}

function openPublicTraceabilityPage() {
    const slug = document.getElementById('public-html-slug').value.trim();
    if (!slug) return alert('Veuillez saisir un slug');
    window.open(`${API_BASE.replace('/api/v1', '').replace(':3030', ':3030')}/t/${slug}`, '_blank');
}
