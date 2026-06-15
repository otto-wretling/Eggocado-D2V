from contextlib import asynccontextmanager
from fastapi import FastAPI
from pydantic import BaseModel
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

normalizer = None


@asynccontextmanager
async def lifespan(app: FastAPI):
    global normalizer
    logger.info("Loading nemo text normalizer...")
    from nemo_text_processing.text_normalization.normalize import Normalizer
    normalizer = Normalizer(input_case="cased", lang="en")
    logger.info("Normalizer ready.")
    yield


app = FastAPI(lifespan=lifespan)


class WordsRequest(BaseModel):
    words: list


@app.post("/normalize_words")
def normalize_words(req: WordsRequest):
    try:
        result = []
        for word in req.words:
            try:
                normalized = normalizer.normalize(word, verbose=False).strip()
            except Exception:
                normalized = word
            result.append({"original": word, "normalized": normalized})
        return {"words": result}
    except Exception as e:
        logger.error(f"Word normalization failed: {e}")
        return {"words": [{"original": w, "normalized": w} for w in req.words]}


@app.get("/health")
def health():
    return {"status": "ok"}
