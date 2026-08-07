const { invoke } = window.__TAURI__.core;

window.addEventListener("DOMContentLoaded", async () => {
  try {
    const history = await invoke("get_history");

    const listEl = document.querySelector("#history-list");

    listEl.innerHTML = "";

    history.forEach((item) => {
      const row = document.createElement("div");
      row.classList.add("history-row");

      const img = document.createElement("img");
      img.src = item.artwork_url;
      img.alt = `${item.track_name} artwork`;

      img.onerror = function () {
        img.onerror = null;

        img.src =
          "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'%3E%3Crect width='24' height='24' fill='%23333333'/%3E%3Cpath fill='%23888888' d='M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z'/%3E%3C/svg%3E";
      };

      const textDiv = document.createElement("div");
      textDiv.classList.add("text-info");

      const trackP = document.createElement("p");
      trackP.classList.add("track-name");
      trackP.textContent = item.track_name;

      const artistP = document.createElement("p");
      artistP.classList.add("artist-name");
      artistP.textContent = item.artist_name;

      const albumP = document.createElement("p");
      albumP.classList.add("album-name");
      albumP.textContent = item.collection_name;

      const genreP = document.createElement("p");
      genreP.classList.add("genre-name");
      genreP.textContent = item.primary_genre;

      textDiv.appendChild(trackP);
      textDiv.appendChild(artistP);
      textDiv.appendChild(albumP);
      textDiv.appendChild(genreP);

      row.appendChild(img);
      row.appendChild(textDiv);

      listEl.appendChild(row);
    });
  } catch (error) {
    console.error("Failed to fetch history:", error);
  }
});
