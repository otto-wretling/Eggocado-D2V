from contextlib import asynccontextmanager
from fastapi import FastAPI
from pydantic import BaseModel
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

normalizer = None
inverse_normalizer = None


@asynccontextmanager
async def lifespan(app: FastAPI):
    global normalizer, inverse_normalizer
    logger.info("Loading nemo text normalizer...")
    from nemo_text_processing.text_normalization.normalize import Normalizer
    normalizer = Normalizer(input_case="cased", lang="en")
    logger.info("Normalizer ready.")
    logger.info("Loading nemo inverse text normalizer...")
    from nemo_text_processing.inverse_text_normalization.inverse_normalize import InverseNormalizer
    inverse_normalizer = InverseNormalizer(lang="en")
    logger.info("Inverse normalizer ready.")
    yield


app = FastAPI(lifespan=lifespan)


class TextRequest(BaseModel):
    text: str


@app.post("/normalize")
def normalize(req: TextRequest):
    try:
        result = normalizer.normalize(req.text, verbose=False)
        return {"text": result}
    except Exception as e:
        logger.error(f"Normalization failed: {e}")
        return {"text": req.text}


@app.post("/inverse_normalize")
def inverse_normalize(req: TextRequest):
    try:
        result = inverse_normalizer.inverse_normalize(req.text, verbose=False)
        return {"text": result}
    except Exception as e:
        logger.error(f"Inverse normalization failed: {e}")
        return {"text": req.text}


@app.get("/health")
def health():
    return {"status": "ok"}
